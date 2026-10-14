//! Windows native print dialog and GDI spooling. Called only by the PDFium owner thread.
use super::*;
use windows_sys::Win32::{
    Foundation::GlobalFree, Graphics::Gdi::*, Storage::Xps::*, UI::Controls::Dialogs::*,
};

pub fn print(pdfium: &Pdfium, bytes: Vec<u8>) -> Result<bool> {
    let doc = pdfium.load_pdf_from_byte_vec(bytes, None)?;
    let pages = doc.pages().len();
    ensure!(
        pages > 0 && pages <= u16::MAX as i32,
        "Invalid print page count"
    );
    // The print dialog owns the returned device context and global memory handles.
    let mut dialog: PRINTDLGW = unsafe { std::mem::zeroed() };
    dialog.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
    dialog.Flags = PD_RETURNDC | PD_NOSELECTION | PD_USEDEVMODECOPIESANDCOLLATE;
    dialog.nMinPage = 1;
    dialog.nMaxPage = pages as u16;
    dialog.nFromPage = 1;
    dialog.nToPage = pages as u16;
    dialog.nCopies = 1;
    let accepted = unsafe { PrintDlgW(&mut dialog) };
    struct Resources(PRINTDLGW);
    impl Drop for Resources {
        fn drop(&mut self) {
            unsafe {
                if !self.0.hDC.is_null() {
                    DeleteDC(self.0.hDC);
                }
                if !self.0.hDevMode.is_null() {
                    GlobalFree(self.0.hDevMode);
                }
                if !self.0.hDevNames.is_null() {
                    GlobalFree(self.0.hDevNames);
                }
            }
        }
    }
    let resources = Resources(dialog);
    let dialog = &resources.0;
    if accepted == 0 {
        let error = unsafe { CommDlgExtendedError() };
        ensure!(error == 0, "Windows print dialog failed ({error})");
        return Ok(false);
    }
    let dc = dialog.hDC;
    ensure!(!dc.is_null(), "Printer did not return a device context");
    let title: Vec<u16> = "OpenPDF document\0".encode_utf16().collect();
    let info = DOCINFOW {
        cbSize: std::mem::size_of::<DOCINFOW>() as i32,
        lpszDocName: title.as_ptr(),
        lpszOutput: std::ptr::null(),
        lpszDatatype: std::ptr::null(),
        fwType: 0,
    };
    ensure!(
        unsafe { StartDocW(dc, &info) } > 0,
        "Printer refused the document"
    );
    let result = (|| -> Result<()> {
        let width = unsafe { GetDeviceCaps(dc, HORZRES as i32) };
        let height = unsafe { GetDeviceCaps(dc, VERTRES as i32) };
        ensure!(width > 0 && height > 0, "Printer has no printable area");
        let (first, last) = if dialog.Flags & PD_PAGENUMS != 0 {
            (
                dialog.nFromPage.max(1) as usize - 1,
                dialog.nToPage.min(pages as u16) as usize,
            )
        } else {
            (0, pages as usize)
        };
        for page in first..last {
            let raster = render(
                &doc,
                RenderKey {
                    document: 0,
                    page,
                    width: 2480,
                    rotation: 0,
                },
            )?;
            let mut pixels = raster.rgba;
            for rgba in pixels.chunks_exact_mut(4) {
                rgba.swap(0, 2);
            }
            let mut bitmap: BITMAPINFO = unsafe { std::mem::zeroed() };
            bitmap.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bitmap.bmiHeader.biWidth = raster.width as i32;
            bitmap.bmiHeader.biHeight = -(raster.height as i32);
            bitmap.bmiHeader.biPlanes = 1;
            bitmap.bmiHeader.biBitCount = 32;
            bitmap.bmiHeader.biCompression = BI_RGB;
            let scale =
                (width as f32 / raster.width as f32).min(height as f32 / raster.height as f32);
            let w = (raster.width as f32 * scale) as i32;
            let h = (raster.height as f32 * scale) as i32;
            ensure!(unsafe { StartPage(dc) } > 0, "Could not start printer page");
            let lines = unsafe {
                StretchDIBits(
                    dc,
                    (width - w) / 2,
                    (height - h) / 2,
                    w,
                    h,
                    0,
                    0,
                    raster.width as i32,
                    raster.height as i32,
                    pixels.as_ptr().cast(),
                    &bitmap,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                )
            };
            ensure!(lines > 0, "Printer could not accept page pixels");
            ensure!(
                unsafe { EndPage(dc) } > 0,
                "Could not complete printer page"
            );
        }
        ensure!(
            unsafe { EndDoc(dc) } > 0,
            "Printer could not complete the job"
        );
        Ok(())
    })();
    if result.is_err() {
        unsafe {
            AbortDoc(dc);
        }
    }
    result?;
    Ok(true)
}
