//! Moldura da janela no Windows: barra de título nativa na cor da marca (DWM) e ícone grande
//! da barra de tarefas.
//!
//! A barra continua sendo do sistema (arrastar, snap layouts, acessibilidade); só as cores
//! mudam. No Windows 10 o DWM recusa os atributos e a barra fica no padrão do sistema.
//!
//! O Tauri só define o ícone pequeno da janela (`ICON_SMALL`). Sem o grande (`ICON_BIG`), a
//! barra de tarefas usa o ícone guardado no cache do Windows para o caminho do `.exe`, que pode
//! ser de um build antigo. Aqui o ícone embutido no próprio executável vira o ícone grande.

/// Azul da marca para a barra (`--accent-hover` do tema, #2348CE) e texto branco.
/// COLORREF é 0x00BBGGRR.
#[cfg(windows)]
const CAPTION: u32 = 0x00CE_4823;
#[cfg(windows)]
const TEXT: u32 = 0x00FF_FFFF;

/// Aplica a marca à janela. Falhas só são registradas: a janela funciona do mesmo jeito.
pub fn apply_brand(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        let Ok(hwnd) = window.hwnd() else {
            tracing::warn!("janela sem HWND; moldura padrão");
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
        if !set_taskbar_icon(raw) {
            tracing::warn!("ícone da barra de tarefas não definido; o Windows usa o do cache");
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}

// `unsafe` só neste arquivo (o crate usa `deny`, o workspace `forbid`). As duas funções abaixo
// são as únicas chamadas FFI do app.

/// Cor de um atributo do DWM. O ponteiro aponta para um `u32` na pilha que vive durante toda a
/// chamada, e o tamanho informado é exatamente o dele; o HWND vem do Tauri para uma janela viva.
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

/// Carrega o ícone do app embutido no executável (recurso 32512, gravado pelo `tauri-build`)
/// no tamanho de ícone grande do sistema e o define como `ICON_BIG`. O nome do recurso é um
/// inteiro (`MAKEINTRESOURCE`), não um ponteiro desreferenciado; o ícone carregado pertence à
/// janela pelo resto da vida do processo.
#[cfg(windows)]
#[allow(unsafe_code)]
fn set_taskbar_icon(hwnd: *mut core::ffi::c_void) -> bool {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, ICON_BIG, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW, SM_CXICON, SM_CYICON,
        SendMessageW, WM_SETICON,
    };
    const APP_ICON_RESOURCE_ID: usize = 32512;
    unsafe {
        let module = GetModuleHandleW(std::ptr::null());
        if module.is_null() {
            return false;
        }
        let icon = LoadImageW(
            module,
            APP_ICON_RESOURCE_ID as *const u16,
            IMAGE_ICON,
            GetSystemMetrics(SM_CXICON),
            GetSystemMetrics(SM_CYICON),
            LR_DEFAULTCOLOR,
        );
        if icon.is_null() {
            return false;
        }
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
    }
    true
}
