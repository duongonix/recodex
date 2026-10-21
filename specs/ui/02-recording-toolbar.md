# UI Spec 02 — Floating Recording Toolbar

## 1. Mục tiêu và tham chiếu

Thanh điều khiển nhỏ nổi trên desktop sau khi user bấm **Start Recording**. Đây không phải một trang của cửa sổ chính: nó là một Tauri window độc lập, có thể kéo và thu gọn, dùng để điều khiển phiên quay.

**Ảnh tham chiếu:** `./images/ui-reference.png` — xem 3 trạng thái bên phải: **Recording toolbar**, **Minimized (small mode)** và **Paused state**. Giữ tinh thần dark + red và thứ tự controls, không bắt chước các nhãn chú thích ngoài thanh.

**Stack:** Tauri 2 + Svelte 5 + Tailwind CSS 4 + shadcn-svelte + lucide-svelte. Tái sử dụng design tokens từ `01-main-window.md`.

## 2. Cửa sổ Tauri

- Window label: `recording-toolbar`.
- Kích thước full đề xuất: **620 × 58 CSS px**; chiều rộng có thể linh hoạt 580–680 px tùy platform/font.
- Compact mode: **270 × 50 CSS px** so the webcam control remains available while compact.
- Borderless, nền `#0E1118` hoặc `#11151D`, bo góc 14–16 px, border 1 px `#303744`, shadow mềm.
- `alwaysOnTop: true` khi đang quay; không chiếm taskbar/dock nếu nền tảng hỗ trợ.
- Có drag region rõ ràng (vùng trống hoặc status group), các nút không được vô tình kéo window.
- Khởi tạo ẩn; chỉ show khi recording backend xác nhận start thành công. Backend đặt toolbar ở góc dưới bên phải của target đang quay. Nếu webcam bật, overlay nằm ngay bên dưới toolbar và căn mép phải với toolbar.
- Trên Windows, sau khi WGC bắt đầu nhưng trước khi hiện toolbar, backend đặt lại `WDA_EXCLUDEFROMCAPTURE` cho HWND và đọc kiểm tra giá trị Windows trả về; không chấp nhận ghi một cửa sổ toolbar thành vùng trống/đen nếu native affinity không được xác nhận.
- Mỗi phiên quay bắt đầu với toolbar và webcam ở góc dưới bên phải của target; webcam nằm thấp hơn toolbar. Khi người dùng kéo toolbar, desktop webcam overlay và webcam trong compositor di chuyển cùng khoảng cách. Vị trí tùy chỉnh chỉ giữ trong phiên hiện tại.
- Không tự động assume toolbar bị loại khỏi recording: dùng native capture exclusion khi có hỗ trợ; fallback phải được thiết kế/test riêng và không hứa tính năng trên mọi OS.

## 3. Visual tokens

- Nền toolbar: `#0E1118`; control surface: `#1B2029`; hover `#282E39`.
- Viền: `#303744`; text chính `#F5F6F8`; text phụ `#A0A8B7`.
- Recording red `#F02D36`; paused orange `#FF9F43`.
- Font time: 14–15 px, tabular numbers để tránh nhảy độ rộng khi timer tăng.
- Button radius 9–10 px; gap 8 px; icon 18–20 px; padding ngang 12–14 px.
- Hiệu ứng nhẹ dưới 180 ms, không làm rung layout; không cần animation record dot nhấp nháy liên tục.
- Ba vùng status, recording controls và window actions căn giữa theo cùng một trục dọc; divider nằm chính giữa chiều cao toolbar và không dùng căn stretch theo mép trên.

## 4. Full toolbar layout — trái sang phải

1. **Status + Timer**: chấm đỏ (`circle-dot` hoặc custom red circle) + `00:00:12`, trong một group cố định độ rộng. Timer dựa trên elapsed time của backend, không dựa vào bộ đếm frontend dễ drift.
2. **Divider**: vạch dọc mảnh.
3. **Pause / Resume**: button có icon `pause` và chữ `Pause` khi recording; chuyển icon `play` và chữ `Resume` khi paused.
4. **Stop**: icon `square` và chữ `Stop`, dùng màu neutral surface, hover nhấn mạnh; bấm để kết thúc và lưu.
5. **Microphone**: icon `mic` / `mic-off` và chevron dropdown tùy chọn thiết bị khi backend hỗ trợ.
6. **System Audio**: icon `volume-2` / `volume-x` và chevron khi có lựa chọn nguồn.
7. **Webcam**: icon camera/camera-off, active state đỏ; bật/tắt webcam overlay qua recording worker và đồng bộ từ backend.
8. **Divider**.
9. **Minimize**: icon `minus`, chuyển sang compact mode.
10. **Close**: icon `x`. Khi recording, không được đóng cửa sổ rồi bỏ mặc phiên quay: mở xác nhận hoặc thực hiện hành vi được ghi rõ (khuyến nghị: chỉ hide toolbar, giữ recording, cho phép mở lại từ tray/hotkey). Không dùng X làm Stop ngầm.

Các control icon-only có tooltip và aria-label; nút disabled phải giải thích nguyên nhân (ví dụ thiết bị không có quyền truy cập).

## 5. Recording state

- Dot đỏ + timer trắng.
- Pause button hiển thị `Pause`.
- Stop luôn sẵn sàng, trừ khi đang stopping.
- Toggling microphone/system audio gửi command đến backend, chỉ đổi UI khi được xác nhận; nếu backend không hỗ trợ hot-toggle, disable và thông báo.
- Webcam toggle gửi command native, hiển thị active state sau khi camera session mở thành công; nếu camera chưa chọn thì control giải thích lý do bị khóa.
- Mỗi nút phải có vùng click ít nhất 36 × 36 px.

## 6. Paused state

- Dot đỏ chuyển thành icon pause màu cam, timer và nhãn `Paused` màu cam (`00:01:45 (Paused)` hoặc tương tự ảnh).
- Timer dừng tăng trong thời gian pause; backend quản lý elapsed active recording time.
- Pause button trở thành `Resume` với icon play.
- Stop vẫn hoạt động.
- Không tự động tiếp tục ghi hình khi người dùng di chuyển cửa sổ hoặc mở dropdown.

## 7. Compact mode

- Toolbar rút gọn còn chấm trạng thái + timer + nút **Webcam** + nút **Stop** + nút **Expand**.
- Kích thước đề xuất 270 × 50 px; không hiển thị audio controls và Pause trong compact mode để giữ gọn theo ảnh tham chiếu.
- Khi paused, compact mode phải có chỉ báo paused rõ ràng bằng icon/màu; không giả vờ đang quay.
- Có cách quay lại full toolbar bằng nút Expand; tooltip và hotkey hỗ trợ.
- Đặt gần mép màn hình nhưng không tự che nút hệ thống; user có thể kéo sang vị trí khác.

## 8. Stop & Save lifecycle

1. User bấm Stop → disable nút Stop, hiện trạng thái `Saving…` nhỏ trong toolbar.
2. Rust backend finalize container/encoder và flush file; frontend chờ kết quả.
3. Thành công → ẩn toolbar, hiện lại main hoặc thông báo `Recording saved` kèm hành động `Open folder`.
4. Thất bại → không báo đã lưu; hiển thị error và recovery path nếu có, tránh xóa dữ liệu chưa finalize.
5. Nếu backend crash, UI phản ánh trạng thái thất bại và cung cấp thông tin khôi phục nếu khả dụng.

## 9. Tauri event/command contract (định hướng)

- Commands: `start_recording(config)`, `pause_recording()`, `resume_recording()`, `stop_recording()`, `set_microphone_enabled(bool)`, `set_system_audio_enabled(bool)`, `get_recording_state()`.
- Events: `recording://state-changed`, `recording://elapsed`, `recording://error`, `recording://saved`.
- Tên chỉ là contract đề xuất, Codex phải đồng bộ với backend hiện hữu nếu đã có API tương đương; không tạo hai nguồn state cạnh tranh.
- Recording state được quản lý tập trung bởi Rust, không phụ thuộc sự tồn tại của cửa sổ toolbar.
- Window show/hide/resize/move phải dùng Tauri Window API; chỉ cấp quyền Tauri cần thiết, không bật quyền toàn cục bừa bãi.
- Toolbar move events cập nhật vị trí webcam tương ứng trong recording compositor; mặc định cả toolbar lẫn webcam ở góc dưới phải của target.

## 10. Hotkeys & khả năng tiếp cận

- Shortcut start/stop/pause chỉ được hiển thị nếu đăng ký thành công; tránh conflict với OS và ứng dụng khác.
- Có đường mở lại toolbar từ system tray hoặc global shortcut nếu đã hide.
- Keyboard focus rõ; Esc đóng dropdown, không tự dừng recording.
- Không phát âm thanh thông báo vào video nếu người dùng không chọn.
- Hiển thị timer có tabular numbers; giữ layout không thay đổi khi số phút tăng.

## 11. Definition of Done

- [ ] Full toolbar giống bố cục ảnh tham chiếu, màu sắc thống nhất main window.
- [ ] Toolbar là cửa sổ Tauri độc lập, luôn nổi khi recording.
- [ ] Có Recording, Paused, Compact, Saving, Error states.
- [ ] Pause/Resume/Stop gọi backend thật và chờ xác nhận.
- [ ] Compact ↔ Full không làm mất recording state.
- [ ] Close/hide không vô tình hủy bản ghi; toolbar có thể mở lại.
- [ ] Timer chính xác, không drift sau pause/resume.
- [ ] Đã kiểm thử behavior khi đổi monitor, DPI scaling và hạn chế capture exclusion theo OS.
