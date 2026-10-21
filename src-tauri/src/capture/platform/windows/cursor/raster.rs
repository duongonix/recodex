use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
pub(super) struct DamageRect {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Ring {
    pub center_x: i32,
    pub center_y: i32,
    pub radius: f32,
    pub thickness: f32,
    pub color: [u8; 4],
    pub alpha: u8,
}

#[derive(Debug, Clone, Copy)]
struct PointerPixel {
    x: i32,
    y: i32,
    color: [u8; 4],
}

const POINTER_POLYGON: [(i32, i32); 7] = [
    (0, 0),
    (0, 40),
    (10, 30),
    (18, 49),
    (26, 45),
    (19, 28),
    (35, 27),
];

pub(super) fn cursor_bounds(x: i32, y: i32, width: u32, height: u32) -> DamageRect {
    let max_x = i64::from(width);
    let max_y = i64::from(height);
    DamageRect {
        left: (i64::from(x) - 30).clamp(0, max_x) as u32,
        top: (i64::from(y) - 30).clamp(0, max_y) as u32,
        right: (i64::from(x) + 44).clamp(0, max_x) as u32,
        bottom: (i64::from(y) + 53).clamp(0, max_y) as u32,
    }
}

pub(super) fn union_rect(first: Option<DamageRect>, second: DamageRect) -> Option<DamageRect> {
    first.map_or(Some(second), |first| {
        Some(DamageRect {
            left: first.left.min(second.left),
            top: first.top.min(second.top),
            right: first.right.max(second.right),
            bottom: first.bottom.max(second.bottom),
        })
    })
}

pub(super) fn rect_is_empty(rect: DamageRect) -> bool {
    rect.left >= rect.right || rect.top >= rect.bottom
}

pub(super) fn pixel_offset(y: u32, x: u32, width: u32) -> usize {
    ((y as usize * width as usize) + x as usize) * 4
}

pub(super) fn draw_pointer(pixels: &mut [u8], width: u32, height: u32, x: i32, y: i32) {
    for pixel in pointer_mask() {
        let target_x = i64::from(x) + i64::from(pixel.x);
        let target_y = i64::from(y) + i64::from(pixel.y);
        if target_x < 0
            || target_y < 0
            || target_x >= i64::from(width)
            || target_y >= i64::from(height)
        {
            continue;
        }
        let offset = pixel_offset(target_y as u32, target_x as u32, width);
        pixels[offset..offset + 4].copy_from_slice(&pixel.color);
    }
}

pub(super) fn draw_ring(pixels: &mut [u8], width: u32, height: u32, ring: Ring) {
    let Ring {
        center_x,
        center_y,
        radius,
        thickness,
        color,
        alpha,
    } = ring;
    if alpha == 0 || width == 0 || height == 0 {
        return;
    }
    let bound = (radius + thickness).ceil() as i32;
    let max_x = i64::from(width) - 1;
    let max_y = i64::from(height) - 1;
    let left = (i64::from(center_x) - i64::from(bound)).clamp(0, max_x) as i32;
    let right = (i64::from(center_x) + i64::from(bound)).clamp(0, max_x) as i32;
    let top = (i64::from(center_y) - i64::from(bound)).clamp(0, max_y) as i32;
    let bottom = (i64::from(center_y) + i64::from(bound)).clamp(0, max_y) as i32;
    if left > right || top > bottom {
        return;
    }
    for y in top..=bottom {
        for x in left..=right {
            let dx = (i64::from(x) - i64::from(center_x)) as f32;
            let dy = (i64::from(y) - i64::from(center_y)) as f32;
            let distance = (dx * dx + dy * dy).sqrt();
            let coverage = ((thickness * 0.5 + 0.6) - (distance - radius).abs()).clamp(0.0, 1.0);
            if coverage > 0.0 {
                blend_pixel(
                    pixels,
                    pixel_offset(y as u32, x as u32, width),
                    color,
                    (f32::from(alpha) * coverage) as u8,
                );
            }
        }
    }
}

fn pointer_mask() -> &'static [PointerPixel] {
    static MASK: OnceLock<Vec<PointerPixel>> = OnceLock::new();
    MASK.get_or_init(|| {
        let mut mask = Vec::new();
        for y in -2..=51 {
            for x in -2..=37 {
                let inside = polygon_contains(x, y);
                let outline = !inside
                    && (-2..=2).any(|dy| (-2..=2).any(|dx| polygon_contains(x + dx, y + dy)));
                if inside || outline {
                    mask.push(PointerPixel {
                        x,
                        y,
                        color: if inside {
                            [0, 0, 0, 255]
                        } else {
                            [255, 255, 255, 255]
                        },
                    });
                }
            }
        }
        mask
    })
}

fn polygon_contains(x: i32, y: i32) -> bool {
    let mut inside = false;
    let pixel_center_x = f64::from(x) + 0.5;
    let pixel_center_y = f64::from(y) + 0.5;
    for (index, &(x1, y1)) in POINTER_POLYGON.iter().enumerate() {
        let (x2, y2) = POINTER_POLYGON[(index + POINTER_POLYGON.len() - 1) % POINTER_POLYGON.len()];
        if (f64::from(y1) > pixel_center_y) != (f64::from(y2) > pixel_center_y)
            && pixel_center_x
                < f64::from(x2 - x1) * (pixel_center_y - f64::from(y1)) / f64::from(y2 - y1)
                    + f64::from(x1)
        {
            inside = !inside;
        }
    }
    inside
}

fn blend_pixel(pixels: &mut [u8], offset: usize, color: [u8; 4], alpha: u8) {
    let inverse = 255_u32 - u32::from(alpha);
    for channel in 0..3 {
        pixels[offset + channel] = ((u32::from(color[channel]) * u32::from(alpha)
            + u32::from(pixels[offset + channel]) * inverse
            + 127)
            / 255) as u8;
    }
    pixels[offset + 3] = 255;
}
