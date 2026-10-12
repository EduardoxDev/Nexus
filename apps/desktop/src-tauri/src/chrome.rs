//! Moldura da janela: barra de título nativa na cor da marca (Windows 11, via DWM).
//!
//! A barra continua sendo do sistema (arrastar, snap layouts, acessibilidade); só as cores
//! mudam. No Windows 10 o DWM recusa os atributos e a barra fica no padrão do sistema.

/// Azul da marca para a barra (`--accent-hover` do tema, #2348CE) e texto branco.
/// COLORREF é 0x00BBGGRR.
#[cfg(windows)]
const CAPTION: u32 = 0x00CE_4823;
#[cfg(windows)]
const TEXT: u32 = 0x00FF_FFFF;

/// Aplica as cores da marca à barra de título. Falhas só são registradas: a janela funciona
/// do mesmo jeito com a barra padrão.
pub fn apply_brand(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        let Ok(hwnd) = window.hwnd() else {
            tracing::warn!("janela sem HWND; barra de título padrão");
            return;
        };
        use windows_sys::Win32::Graphics::Dwm::{
            DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
        };
        let raw = hwnd.0;
        let ok = [
            (DWMWA_CAPTION_COLOR, CAPTION),
            (DWMWA_BORDER_COLOR, CAPTION),
            (DWMWA_TEXT_COLOR, TEXT),
        ]
        .into_iter()
        .all(|(attribute, color)| set_dwm_color(raw, attribute, color));
        if !ok {
            tracing::info!("cores da barra de título não suportadas por este Windows");
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}

/// Única chamada FFI do app. `unsafe` só aqui: o ponteiro aponta para um `u32` na pilha que
/// vive durante toda a chamada, e o tamanho informado é exatamente o dele; o HWND vem do
/// próprio Tauri para uma janela viva.
#[cfg(windows)]
#[allow(unsafe_code)]
fn set_dwm_color(hwnd: *mut core::ffi::c_void, attribute: i32, color: u32) -> bool {
    use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
    let value = color;
    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            attribute as u32,
            std::ptr::from_ref(&value).cast(),
            size_of::<u32>() as u32,
        )
    };
    hr >= 0
}
