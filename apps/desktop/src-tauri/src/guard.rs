//! Barreiras do webview: a janela só exibe o próprio app.
//!
//! Mesmo com CSP estrita, um link clicado, um redirecionamento ou um `window.open` poderiam
//! levar o webview (que tem acesso ao IPC) para outra origem. Aqui toda navegação para fora
//! do app é recusada e nenhuma janela nova é criada.

use tauri::Url;

/// A URL pertence ao app? Em produção: `tauri://localhost` ou `http(s)://tauri.localhost`
/// (Windows). Em desenvolvimento, também o servidor do Vite configurado em `devUrl`.
pub fn is_app_url(url: &Url, dev_url: Option<&Url>) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => {
            url.host_str() == Some("tauri.localhost")
                || dev_url.is_some_and(|dev| {
                    dev.scheme() == url.scheme()
                        && dev.host_str() == url.host_str()
                        && dev.port_or_known_default() == url.port_or_known_default()
                })
        }
        // Documento vazio inicial do WebView2.
        "about" => url.as_str() == "about:blank",
        _ => false,
    }
}

/// Decide uma navegação e registra as recusas (sem a URL completa: pode conter dados).
pub fn allow_navigation(url: &Url, dev_url: Option<&Url>) -> bool {
    let ok = is_app_url(url, dev_url);
    if !ok {
        tracing::warn!(
            scheme = url.scheme(),
            host = url.host_str().unwrap_or(""),
            "navegação para fora do app bloqueada"
        );
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn app_origins_are_allowed() {
        assert!(is_app_url(&url("tauri://localhost/index.html"), None));
        assert!(is_app_url(&url("http://tauri.localhost/"), None));
        assert!(is_app_url(
            &url("https://tauri.localhost/assets/x.js"),
            None
        ));
        assert!(is_app_url(&url("about:blank"), None));
    }

    #[test]
    fn everything_else_is_blocked() {
        for u in [
            "https://example.com/",
            "http://tauri.localhost.evil.com/",
            "http://evil.com/?tauri.localhost",
            "tauri://evil/",
            "file:///C:/Windows/win.ini",
            "javascript:alert(1)",
            "data:text/html,<script>1</script>",
            "about:srcdoc",
            "http://127.0.0.1:1420/",
        ] {
            assert!(!is_app_url(&url(u), None), "{u} deveria ser bloqueada");
        }
    }

    #[test]
    fn dev_server_only_with_exact_origin() {
        let dev = url("http://127.0.0.1:1420");
        assert!(is_app_url(
            &url("http://127.0.0.1:1420/src/main.tsx"),
            Some(&dev)
        ));
        assert!(!is_app_url(&url("http://127.0.0.1:1421/"), Some(&dev)));
        assert!(!is_app_url(&url("https://127.0.0.1:1420/"), Some(&dev)));
        assert!(!is_app_url(&url("http://localhost:1420/"), Some(&dev)));
    }
}
