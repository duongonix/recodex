use std::{mem::size_of, slice};

use windows::{
    Win32::Graphics::{
        Direct3D::{Fxc::D3DCompile, ID3DBlob, ID3DInclude},
        Direct3D11::{
            D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_UNORDERED_ACCESS, D3D11_BUFFER_DESC,
            D3D11_CPU_ACCESS_READ, D3D11_FILTER_MIN_MAG_LINEAR_MIP_POINT, D3D11_MAP_READ,
            D3D11_MAPPED_SUBRESOURCE, D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_CLAMP,
            D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11Buffer,
            ID3D11ComputeShader, ID3D11Device, ID3D11DeviceContext, ID3D11SamplerState,
            ID3D11Texture2D, ID3D11UnorderedAccessView,
        },
        Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
    },
    core::PCSTR,
};
use windows_capture::frame::Frame;

use super::CropRect;

const ZOOM_SHADER: &str = r#"
Texture2D<float4> Source : register(t0);
SamplerState LinearClamp : register(s0);
RWTexture2D<float4> Destination : register(u0);

cbuffer ZoomParameters : register(b0) {
    float2 CropOrigin;
    float2 CropSize;
    uint2 OutputSize;
    uint2 Padding;
};

[numthreads(16, 16, 1)]
void main(uint3 id : SV_DispatchThreadID) {
    if (id.x >= OutputSize.x || id.y >= OutputSize.y) return;
    float2 outputPosition = float2(id.xy) + 0.5;
    float2 uv = (CropOrigin + outputPosition * CropSize / float2(OutputSize)) / float2(OutputSize);
    float4 color = Source.SampleLevel(LinearClamp, uv, 0);
    // The output texture is RGBA but the recorder contract consumes packed BGRA bytes.
    Destination[id.xy] = color.bgra;
}
"#;

#[repr(C)]
struct ZoomConstants {
    crop_origin: [f32; 2],
    crop_size: [f32; 2],
    output_size: [u32; 2],
    padding: [u32; 2],
}

pub(super) struct GpuZoomRenderer {
    output: ID3D11Texture2D,
    output_uav: ID3D11UnorderedAccessView,
    staging: ID3D11Texture2D,
    constants: ID3D11Buffer,
    shader: ID3D11ComputeShader,
    sampler: ID3D11SamplerState,
    width: u32,
    height: u32,
}

impl GpuZoomRenderer {
    pub(super) fn new(device: &ID3D11Device, width: u32, height: u32) -> Result<Self, String> {
        if width < 2 || height < 2 {
            return Err("The capture frame is too small to zoom.".to_owned());
        }
        let output_desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_UNORDERED_ACCESS.0 as u32,
            ..Default::default()
        };
        let mut output = None;
        let mut output_uav = None;
        let mut staging = None;
        let mut constants = None;
        let mut shader = None;
        let mut sampler = None;

        unsafe {
            device
                .CreateTexture2D(&output_desc, None, Some(&mut output))
                .map_err(|error| format!("Could not create the zoom output texture: {error}"))?;
            let output =
                output.ok_or_else(|| "D3D11 returned no zoom output texture.".to_owned())?;
            device
                .CreateUnorderedAccessView(&output, None, Some(&mut output_uav))
                .map_err(|error| format!("Could not create the zoom output view: {error}"))?;

            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                ..Default::default()
            };
            device
                .CreateTexture2D(&staging_desc, None, Some(&mut staging))
                .map_err(|error| format!("Could not create the zoom readback texture: {error}"))?;

            let constant_desc = D3D11_BUFFER_DESC {
                ByteWidth: size_of::<ZoomConstants>() as u32,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                ..Default::default()
            };
            device
                .CreateBuffer(&constant_desc, None, Some(&mut constants))
                .map_err(|error| format!("Could not create the zoom parameter buffer: {error}"))?;

            let bytecode = compile_zoom_shader()?;
            device
                .CreateComputeShader(&bytecode, None, Some(&mut shader))
                .map_err(|error| format!("Could not create the zoom compute shader: {error}"))?;

            let sampler_desc = D3D11_SAMPLER_DESC {
                Filter: D3D11_FILTER_MIN_MAG_LINEAR_MIP_POINT,
                AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                MipLODBias: 0.0,
                MaxAnisotropy: 1,
                ComparisonFunc: windows::Win32::Graphics::Direct3D11::D3D11_COMPARISON_NEVER,
                BorderColor: [0.0; 4],
                MinLOD: 0.0,
                MaxLOD: 0.0,
            };
            device
                .CreateSamplerState(&sampler_desc, Some(&mut sampler))
                .map_err(|error| format!("Could not create the zoom sampling state: {error}"))?;

            Ok(Self {
                output,
                output_uav: output_uav
                    .ok_or_else(|| "D3D11 returned no zoom output view.".to_owned())?,
                staging: staging
                    .ok_or_else(|| "D3D11 returned no zoom readback texture.".to_owned())?,
                constants: constants
                    .ok_or_else(|| "D3D11 returned no zoom parameter buffer.".to_owned())?,
                shader: shader
                    .ok_or_else(|| "D3D11 returned no zoom compute shader.".to_owned())?,
                sampler: sampler.ok_or_else(|| "D3D11 returned no zoom sampler.".to_owned())?,
                width,
                height,
            })
        }
    }

    pub(super) fn process(&mut self, frame: &Frame, crop: CropRect) -> Result<Vec<u8>, String> {
        if frame.width() != self.width || frame.height() != self.height {
            return Err("The capture dimensions changed during zoom processing.".to_owned());
        }
        if frame.desc().Format != DXGI_FORMAT_B8G8R8A8_UNORM {
            return Err("The GPU zoom path requires BGRA8 capture frames.".to_owned());
        }
        if frame.desc().BindFlags
            & windows::Win32::Graphics::Direct3D11::D3D11_BIND_SHADER_RESOURCE.0 as u32
            == 0
        {
            return Err("The WGC texture cannot be bound as a D3D11 shader resource.".to_owned());
        }

        let mut source_view = None;
        unsafe {
            frame
                .device()
                .CreateShaderResourceView(frame.as_raw_texture(), None, Some(&mut source_view))
                .map_err(|error| format!("Could not create the source frame view: {error}"))?;
        }
        let source_view =
            source_view.ok_or_else(|| "D3D11 returned no source frame view.".to_owned())?;
        let constants = ZoomConstants {
            crop_origin: [crop.left, crop.top],
            crop_size: [crop.width, crop.height],
            output_size: [self.width, self.height],
            padding: [0, 0],
        };
        let context: &ID3D11DeviceContext = frame.device_context();
        unsafe {
            context.UpdateSubresource(
                &self.constants,
                0,
                None,
                (&constants as *const ZoomConstants).cast(),
                0,
                0,
            );
            context.CSSetShader(Some(&self.shader), None);
            context.CSSetShaderResources(0, Some(&[Some(source_view)]));
            context.CSSetUnorderedAccessViews(0, 1, Some(&Some(self.output_uav.clone())), None);
            context.CSSetConstantBuffers(0, Some(&[Some(self.constants.clone())]));
            context.CSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            context.Dispatch(self.width.div_ceil(16), self.height.div_ceil(16), 1);
            context.CSSetShaderResources(0, Some(&[None]));
            context.CSSetUnorderedAccessViews(0, 1, Some(&None), None);
            context.CSSetShader(None::<&ID3D11ComputeShader>, None);
            context.CopyResource(&self.staging, &self.output);
        }
        self.read_staging(context)
    }

    fn read_staging(&self, context: &ID3D11DeviceContext) -> Result<Vec<u8>, String> {
        let row_bytes = usize::try_from(self.width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or_else(|| "The zoom frame row size overflowed.".to_owned())?;
        let total_bytes = row_bytes
            .checked_mul(self.height as usize)
            .ok_or_else(|| "The zoom frame size overflowed.".to_owned())?;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            context
                .Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .map_err(|error| format!("Could not map the zoom result: {error}"))?;
        }

        let row_pitch = mapped.RowPitch as usize;
        if mapped.pData.is_null() || row_pitch < row_bytes {
            unsafe { context.Unmap(&self.staging, 0) };
            return Err("D3D11 returned an invalid zoom readback layout.".to_owned());
        }
        let mut pixels = Vec::with_capacity(total_bytes);
        for row in 0..self.height as usize {
            let row_pointer = unsafe { (mapped.pData as *const u8).add(row * row_pitch) };
            let row_pixels = unsafe { slice::from_raw_parts(row_pointer, row_bytes) };
            pixels.extend_from_slice(row_pixels);
        }
        unsafe { context.Unmap(&self.staging, 0) };
        Ok(pixels)
    }
}

fn compile_zoom_shader() -> Result<Vec<u8>, String> {
    let mut compiled: Option<ID3DBlob> = None;
    unsafe {
        D3DCompile(
            ZOOM_SHADER.as_ptr().cast(),
            ZOOM_SHADER.len(),
            PCSTR::null(),
            None,
            None::<&ID3DInclude>,
            PCSTR(c"main".as_ptr().cast()),
            PCSTR(c"cs_5_0".as_ptr().cast()),
            0,
            0,
            &mut compiled,
            None,
        )
        .map_err(|error| format!("Could not compile the D3D11 zoom shader: {error}"))?;
    }
    let compiled = compiled.ok_or_else(|| "D3DCompile returned no shader bytecode.".to_owned())?;
    let size = unsafe { compiled.GetBufferSize() };
    let pointer = unsafe { compiled.GetBufferPointer() as *const u8 };
    if size == 0 || pointer.is_null() {
        return Err("D3DCompile returned empty shader bytecode.".to_owned());
    }
    Ok(unsafe { slice::from_raw_parts(pointer, size) }.to_vec())
}
