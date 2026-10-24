# RecoDex: 

### Ứng dụng desktop để quay màn hình, chụp ảnh và xem lại bản ghi.

**Chọn màn hình, cửa sổ hoặc vùng cần ghi; thêm âm thanh và webcam; rồi xem lại, chỉnh sửa và xuất video ngay trong RecoDex.**

[![Latest release](https://img.shields.io/github/v/release/duongonix/recodex?style=flat-square&color=cb3837)](https://github.com/duongonix/recodex/releases/latest)
[![Release build](https://img.shields.io/github/actions/workflow/status/duongonix/recodex/release.yml?style=flat-square&label=release%20build)](https://github.com/duongonix/recodex/actions/workflows/release.yml)
[![Windows capture](https://img.shields.io/badge/Windows-capture%20supported-0078D6?style=flat-square&logo=windows)](https://github.com/duongonix/recodex/releases/latest)
[![MIT License](https://img.shields.io/github/license/duongonix/recodex?style=flat-square&color=cb3837)](https://github.com/duongonix/recodex/blob/main/LICENSE)

![Giao diện RecoDex](static/recodex-preview.png)

## Giới thiệu

RecoDex là ứng dụng desktop dành cho người cần tạo video hướng dẫn, lưu lại cuộc họp hoặc thao tác, báo cáo lỗi bằng ảnh chụp, và chỉnh sửa nhanh bản ghi trước khi chia sẻ. Bạn có thể chọn đúng nội dung cần ghi, theo dõi tiến trình bằng thanh điều khiển nổi, rồi xem lại hoặc xuất kết quả.

Ứng dụng hiện hỗ trợ đầy đủ luồng quay và chụp trên **Windows**. Các gói dành cho macOS và Linux có thể xuất hiện trong trang phát hành, nhưng chức năng quay/chụp màn hình trên hai hệ điều hành này chưa được hỗ trợ.

## Chức năng

### Quay màn hình

- Quay toàn bộ màn hình, một cửa sổ đang mở hoặc vùng màn hình do bạn chọn.
- Tạm dừng, tiếp tục và dừng quay từ thanh công cụ nổi; xem thời lượng quay ngay trên thanh công cụ.
- Thu âm microphone, âm thanh hệ thống hoặc cả hai. Khi bật đồng thời, âm thanh được trộn vào video để phát thuận tiện trong trình phát thông thường.
- Chọn chất lượng và tốc độ khung hình phù hợp với màn hình và máy tính. Có các lựa chọn đến 2160p và 60 FPS khi nguồn ghi hỗ trợ.
- Bật webcam overlay khi cần xuất hiện trong video. Webcam mặc định tắt; khi bật, hình webcam được đặt cạnh thanh công cụ và di chuyển cùng thanh công cụ.
- Dùng hiệu ứng con trỏ lớn, màu đen và hiệu ứng nhấn chuột trong bản ghi. Có thể cấu hình phím giữ để phóng to theo con trỏ trong lúc quay màn hình hoặc cửa sổ.
- Xem lại video sau khi quay nếu bật tùy chọn preview trong Settings. Khi tắt preview, bản ghi được lưu ngay sau khi kết thúc.

### Phím tắt

Khi RecoDex đang chạy, các phím tắt cho phép bắt đầu nhanh từng thao tác:

| Thao tác | Phím tắt mặc định trên Windows |
|---|---|
| Quay toàn màn hình | `Ctrl` + `Shift` + `Alt` + `1` |
| Quay cửa sổ | `Ctrl` + `Shift` + `Alt` + `2` |
| Quay vùng chọn | `Ctrl` + `Shift` + `Alt` + `3` |
| Chụp toàn màn hình | `Ctrl` + `Shift` + `Alt` + `4` |
| Chụp cửa sổ | `Ctrl` + `Shift` + `Alt` + `5` |
| Chụp vùng chọn | `Ctrl` + `Shift` + `Alt` + `6` |
| Mở RecoDex hoặc hiện phiên đang chạy | `Ctrl` + `Alt` + `Shift` + `R` |

Các phím tắt được đăng ký khi ứng dụng mở. Nếu tổ hợp đang được ứng dụng khác sử dụng, Settings sẽ báo trạng thái không khả dụng.

### Chụp và chỉnh sửa ảnh

- Chụp màn hình, cửa sổ hoặc vùng chọn.
- Lưu ảnh dưới dạng PNG hoặc JPEG vào thư mục bạn chọn, hoặc sao chép ảnh vào clipboard.
- Xem trước ảnh và dùng các công cụ chỉnh sửa như crop, bút vẽ, mũi tên và chú thích trước khi lưu.

### Webcam overlay

- Bật hoặc tắt webcam trong Settings. Webcam tắt mặc định và chỉ khởi động khi bạn bật tính năng hoặc mở preview.
- Chọn camera đang kết nối và xem hình ảnh trực tiếp trước khi quay.
- Điều chỉnh hình dạng, viền, độ mờ, phản chiếu ngang và nền webcam.
- Khi bắt đầu quay với webcam bật, webcam xuất hiện trong video cùng thanh công cụ. Có thể kéo thanh công cụ để di chuyển cả cụm.

### Xem lại và chỉnh sửa video

- Phát hoặc tạm dừng bản ghi, kéo playhead để xem một thời điểm cụ thể và điều chỉnh âm lượng.
- Dùng timeline để chia clip, cắt đầu/cuối, xóa đoạn đã chọn và hoàn tác hoặc làm lại thao tác.
- Xuất video đã chỉnh sửa thành tệp MP4 mà không thay đổi bản ghi gốc.

### Thư viện bản ghi

- Xem lại các video và ảnh chụp đã lưu trong cùng một danh sách.
- Lọc theo loại nội dung, tìm kiếm theo tên tệp, mở tệp bằng ứng dụng mặc định hoặc mở thư mục chứa tệp.
- Ảnh xem trước được tạo từ nội dung đã lưu; các tệp hiện có trong thư mục đầu ra cũng được hiển thị.

### Cài đặt và cập nhật

- Chọn thư mục lưu video và ảnh chụp.
- Cấu hình microphone, âm thanh hệ thống, webcam, chất lượng video, tốc độ khung hình và xem trước sau khi quay.
- Xem các phím tắt cùng trạng thái hoạt động trong Settings.
- Khi có phiên bản mới, biểu tượng cập nhật sẽ xuất hiện. Mở trang thông tin phiên bản để đọc nội dung phát hành, sau đó nhấn **Update** nếu muốn cài đặt.

## Cài đặt

Tải phiên bản mới nhất từ [GitHub Releases](https://github.com/duongonix/recodex/releases/latest). Chọn gói phù hợp với hệ điều hành và loại bộ xử lý của thiết bị.

### Windows

1. Tải bộ cài `.exe` phù hợp với máy tính. Hầu hết máy tính Windows dùng gói **x64**; chọn **ARM64** nếu mục *System type* trong Settings → System → About cho biết thiết bị dùng bộ xử lý ARM.
2. Mở tệp đã tải xuống và làm theo hướng dẫn trên màn hình.
3. Mở RecoDex từ Start menu.
4. Khi Windows hỏi quyền truy cập microphone hoặc camera, cho phép nếu bạn muốn thu âm hoặc dùng webcam. Bạn có thể thay đổi lựa chọn này trong Windows Settings.

### macOS

1. Tải tệp `.dmg` dành cho máy Mac Intel hoặc Apple Silicon.
2. Mở tệp đã tải xuống và kéo RecoDex vào thư mục **Applications**.
3. Mở RecoDex từ Applications.

> Gói macOS hiện chưa hỗ trợ quay hoặc chụp màn hình.

### Linux

1. Tải gói phù hợp với bản phân phối của bạn: **AppImage**, **DEB** hoặc **RPM**. Chọn x64 hoặc ARM64 theo kiến trúc thiết bị.
2. Với AppImage, mở thuộc tính tệp và bật quyền chạy chương trình, sau đó mở tệp. Với DEB hoặc RPM, mở gói bằng trình cài đặt ứng dụng của bản phân phối.
3. Mở RecoDex từ danh sách ứng dụng.

> Gói Linux hiện chưa hỗ trợ quay hoặc chụp màn hình.

### Cập nhật ứng dụng

Khi RecoDex phát hiện phiên bản mới, chọn biểu tượng cập nhật cạnh nút Settings. Trang cập nhật hiển thị phiên bản mới và ghi chú phát hành. Nhấn **Update** để tải và cài đặt; RecoDex sẽ khởi động lại khi cập nhật hoàn tất.
