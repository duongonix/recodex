# UI Spec 01 — Compact Main Window

## 1. Mục tiêu và tài liệu tham chiếu

Thiết kế cửa sổ mở đầu của ứng dụng quay/chụp màn hình desktop. Giao diện phải nhỏ gọn, tối giản, thao tác được ngay, không có sidebar, không có dashboard phức tạp.

**Ảnh tham chiếu:** `./images/ui-reference.png` — xem khung **Main window (when app opens)** bên trái. Đây là chuẩn về bố cục, màu sắc và cảm giác thị giác; không dùng ảnh làm UI thực tế. Văn bản và icon phải được render bằng component.

**Stack bắt buộc:** Tauri 2, Svelte 5 (runes), Tailwind CSS 4, shadcn-svelte, lucide-svelte. Không tự xây lại component nếu thư viện đã có component tương đương.

## 2. Cửa sổ desktop

- Tauri window label: `main`.
- Kích thước thiết kế: **400 × 490 CSS px**; kích thước này phải chứa toàn bộ layout chính mà không cuộn.
- Không có scrollbar trong main window. Mode tabs nằm ngay dưới titlebar, giữ nguyên vị trí khi đổi Record/Screenshot; phần options thay đổi trong cùng vùng layout.
- Không dùng dòng trạng thái ở footer. Kết quả và lỗi của thao tác capture dùng system notification; development mode ghi cùng thông tin vào console.
- Non-resizable cho MVP, cho phép kéo cửa sổ qua vùng titlebar.
- Cửa sổ không trong suốt toàn bộ; nền dark đồng nhất, viền mỏng và bo góc ~16 px nếu platform cho phép.
- Dùng custom titlebar nếu đáp ứng được kéo cửa sổ và các nút cửa sổ trên cả ba nền tảng; nếu chưa ổn định, ưu tiên native decorations và giữ nội dung bên trong nhất quán.
- Hành vi đóng/ẩn main theo thiết lập Minimize to system tray: mặc định bật khi tray khả dụng; khi đang quay, đóng main không được dừng phiên quay.
- Cửa sổ chính không xuất hiện trong vùng quay khi recording bắt đầu.

## 3. Design tokens

| Token | Giá trị gợi ý | Vai trò |
|---|---|---|
| `--bg` | `#0E1118` | Nền cửa sổ |
| `--surface` | `#171C25` | Cards / controls |
| `--surface-hover` | `#202633` | Hover |
| `--border` | `#2B323E` | Border, divider |
| `--accent` | `#F02D36` | Primary red |
| `--accent-hover` | `#D92731` | Hover red |
| `--text` | `#F5F6F8` | Primary text |
| `--muted` | `#A0A8B7` | Secondary text |
| `--disabled` | `#626B7A` | Disabled |
| `--warning` | `#FF9F43` | Cảnh báo |

- Font: ưu tiên system UI (Inter nếu đã có sẵn); text body 13–14 px; section label 12–13 px; title 14–15 px medium.
- Spacing theo bội số 4 px. Padding outer 16 px; gap section 14–18 px; gap control 8–12 px.
- Border radius: window 16 px, panel 10–12 px, button 10–12 px.
- Bóng đổ nhẹ, không glow đỏ quá mức, không gradient rực rỡ.
- Icon: `lucide-svelte`, stroke 1.75–2 px, size 16–22 px; icon red khi selected, màu muted khi bình thường.

## 4. Layout từ trên xuống

### 4.1 Titlebar — ~42 px

- Trái: app icon đỏ viền tròn + tên ứng dụng (dùng tên sản phẩm thực tế từ config; `ScreenRec` chỉ là placeholder của ảnh demo).
- Phải: nút Settings (gear), minimize và close nếu custom titlebar được bật.
- Click Settings thay nội dung recorder bằng Settings ngay trong cùng cửa sổ 400 × 490; nút Back khôi phục recorder và lựa chọn trước đó.
- Titlebar là drag region, trừ các nút tương tác.

### 4.2 Mode tabs — ~56 px

- Hai tab ngang bằng nhau: **Record** (icon `video`) và **Screenshot** (icon `camera`).
- Active: nền đỏ sẫm trong suốt, viền đỏ rõ, icon đỏ, text trắng. Inactive và active giữ cùng kích thước viền để chuyển tab không làm nút nhúc nhích.
- Inactive: surface dark, text trắng xám; hover nâng nhẹ độ sáng.
- Mặc định active `Record`. Chuyển tab không xóa lựa chọn capture area nếu có thể dùng chung.

### 4.3 Capture Area — label + 3 cards

- Label `Capture Area` màu muted.
- Ba card ngang cùng kích thước: **Full Screen** (`monitor`), **Window** (`app-window`), **Region** (`scan`).
- Card cao ~82 px, icon ở trên, label ở dưới, center aligned.
- Selected card: border 1.5 px accent, background đỏ rất nhạt; unselected: surface.
- Mặc định `Full Screen`.
- Full Screen: nếu nhiều monitor, chọn monitor bằng selector hoặc dialog sau khi bấm Start.
- Window: mở picker chọn cửa sổ khi bấm Start hoặc khi chọn chế độ; phải thể hiện trạng thái không có cửa sổ khả dụng.
- Region: khi bấm Start, ẩn main rồi mở overlay để người dùng kéo vùng chọn; `Esc` hủy và trở về main.

### 4.4 Audio toggles — 2 ô ngang

- Ô trái: icon `volume-2`, label `System Audio`, switch bên phải.
- Ô phải: icon `mic`, label `Microphone`, switch bên phải.
- Mỗi ô surface riêng, cao khoảng 48 px.
- Switch ON: accent red; OFF: dark gray.
- Không giả vờ hỗ trợ audio capture trên platform chưa có. Nếu backend báo unsupported/permission denied, disable switch, hiển thị tooltip/feedback rõ lý do.
- Khi chọn Screenshot, ẩn audio toggles và các mục video; thay bằng tuỳ chọn `Copy to clipboard` và `Save to file` nếu cần, giữ UI gọn.

### 4.5 Recording options — 2 hàng

- Hàng 1: icon `monitor`, label `Quality`, select phía phải (720p, 1080p, 2160p UHD nếu backend hỗ trợ).
- Hàng 2: icon `gauge`, label `FPS`, select phía phải (30, 60; 60 chỉ khả dụng khi engine hỗ trợ).
- Mặc định đề xuất: 1080p / 30 FPS để giảm tải, có thể lưu lựa chọn gần nhất.
- Select có caret xuống; dùng component Select từ shadcn-svelte, không tự tạo popup select.
- Khi mode Screenshot, thay options bằng `Format: PNG / JPG` và `Destination` nếu cần.

### 4.6 Primary action — ~50 px

- Full width, nền đỏ `--accent`, chữ trắng, radius 10–12 px.
- Record mode: icon `circle-dot` + **Start Recording**, shortcut hiển thị nhỏ ở mép phải nếu đã đăng ký thành công (ví dụ `Ctrl + Shift + R`, phải xử lý xung đột và khác biệt platform).
- Screenshot mode: icon `camera` + **Take Screenshot**.
- Hover darken nhẹ; pressed scale rất nhỏ; disabled có lý do; loading khi đang xin quyền/chọn nguồn/khởi động capture.
- Không chuyển sang trạng thái đang quay trước khi backend xác nhận đã bắt đầu capture.

## 5. Luồng tương tác

1. App mở → `main` hiện, mode Record mặc định, Full Screen selected.
2. User chọn source, audio, quality và FPS → settings được cập nhật và persist.
3. Bấm Start → validate quyền/thiết bị, chọn source nếu cần → backend start recording.
4. Khi backend xác nhận success → ẩn `main`, hiện `recording-toolbar` (tham chiếu `02-recording-toolbar.md`).
5. Nếu start thất bại hoặc user hủy picker → giữ/trả lại `main`, hiển thị thông báo lỗi có thể xử lý.
6. Screenshot mode → capture → copy/lưu theo lựa chọn → thông báo ngắn gọn; không mở editor lớn ở MVP.

- Kiểm tra mức tín hiệu microphone/system audio nằm trong `Settings > Snap`; hộp kiểm tra trước khi quay chỉ hiển thị trạng thái sẵn sàng ghi hình.

## 6. State cần thiết

- `mode`: `record | screenshot`
- `captureTarget`: `screen | window | region`
- `systemAudioEnabled`, `microphoneEnabled`
- `quality`: `720p | 1080p | 2160p`
- `fps`: `30 | 60`
- Hold-to-zoom enabled state and shortcut are persisted under Settings > Snap > Recording; capability data gates Windows display/window use.
- `recordingStatus`: `idle | preparing | recording | paused | stopping | error`
- `permissionStatus` và `captureCapabilities` từ backend, không hardcode platform support.
- State recording phải do Rust/backend làm source of truth; frontend subscribe event và sync sau khi reload window.

## 7. Accessibility & hành vi

- Tab/checkbox/select/button có focus ring rõ, điều khiển được bằng keyboard.
- Tooltip/icon button có aria-label; không dựa vào màu sắc duy nhất để báo trạng thái.
- Tối thiểu hit target 32–36 px; text không bị cắt ở hệ thống font scaling thông dụng.
- Animation 120–180 ms; hỗ trợ reduced motion.
- Không hiển thị toolbar hoặc main trong bản ghi nếu API cho phép loại trừ; nếu không, thông báo giới hạn và cung cấp chế độ phù hợp.

## 8. Definition of Done

- [ ] Bố cục tương đương phần main window trong `ui-reference.png`.
- [ ] Tất cả mode/cards/toggles/selects có trạng thái tương tác thực, không chỉ là mockup.
- [ ] Mở app nhanh, UI không bị chặn bởi capture/encode.
- [ ] Start Recording gọi Tauri command thực và xử lý success/error.
- [ ] Khi recording bắt đầu, main ẩn và toolbar hiện đúng trạng thái.
- [ ] Chạy được ở DPI scaling phổ biến; kiểm tra Windows/macOS/Linux, ghi nhận khác biệt API.
