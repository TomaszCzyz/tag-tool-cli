use base64::engine::general_purpose;
use base64::Engine;
use log::{error, info, warn};
use rand::rngs::OsRng;
use rand::TryRngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[allow(unused)]
#[derive(Deserialize)]
struct TokenSuccess {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: Option<String>,
    scope: Option<String>,
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
    error_description: Option<String>,
}

struct OneShotHttpListener {
    listener: TcpListener,
}

impl OneShotHttpListener {
    fn start_new() -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(("127.0.0.1", 38924))?;
        listener.set_nonblocking(false)?;

        Ok(Self { listener })
    }

    fn handle_connection(mut stream: TcpStream) -> std::io::Result<String> {
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf)?;
        let req = String::from_utf8_lossy(&buf[..n]);

        // Very small HTTP parser: first line like:
        // GET /callback?code=...&state=... HTTP/1.1
        let first_line = req.lines().next().unwrap_or_default();
        let path_qs = first_line.split_whitespace().nth(1).unwrap_or("/");

        let html_response = "\
HTTP/1.1 200 OK\r\n\
Content-Type: text/html; charset=utf-8\r\n\
Connection: close\r\n\
\r\n\
<!DOCTYPE html><html><body>\
Login complete. You may close this window and return to the CLI.\
</body></html>";
        let _ = stream.write_all(html_response.as_bytes());
        let _ = stream.flush();

        Ok(path_qs.to_string())
    }

    fn accept(&mut self) -> std::io::Result<(Option<String>, Option<String>)> {
        info!("Listening for redirect at: {}", "<redirect_uri>");
        self.listener.set_nonblocking(false)?;
        self.listener.set_ttl(60).ok();

        // Simple accept loop (single request)
        let (stream, _addr) = self.listener.accept()?;
        let path_qs = OneShotHttpListener::handle_connection(stream)?;

        let query = path_qs.splitn(2, '?').nth(1).unwrap_or("");
        let mut code_opt: Option<String> = None;
        let mut state_opt: Option<String> = None;
        for (k, v) in form_urlencoded::parse(query.as_bytes()) {
            match &*k {
                "code" => code_opt = Some(v.into_owned()),
                "state" => state_opt = Some(v.into_owned()),
                _ => {}
            }
        }

        Ok((code_opt, state_opt))
    }
}

pub(crate) struct LoginFlow {
    domain: String,
    redirect_uri: String,
    scope: String,
    audience: String,
    client_id: String,

    code_verifier: String,
    code_challenge: String,
    state: String,
}

impl LoginFlow {
    pub(crate) fn new() -> Self {
        let (code_verifier, code_challenge, state) = generate_secrets();

        Self {
            domain: "https://dev-y5nz4h20ek8xt3ux.us.auth0.com".to_string(),
            redirect_uri: "http://127.0.0.1:38924/callback".to_string(),
            scope: "profile email".to_string(),
            audience: "https://APP_NAME.azurewebsites.net/api/tag-tool-sync-az-function".to_string(),
            client_id: "JIRCkfkriTf21gGMo6BeQSdIUx5IeUdC".to_string(),
            code_verifier,
            code_challenge,
            state,
        }
    }

    pub(crate) fn run(&self) -> color_eyre::Result<()> {
        let mut listener = OneShotHttpListener::start_new().unwrap();

        self.authorize_user();

        let (code_opt, state_opt) = listener.accept()?;

        let code = match (code_opt, state_opt) {
            (Some(code), Some(returned_state)) if returned_state == self.state => code,
            (Some(_), Some(_)) => {
                error!("State mismatch. Aborting.");
                return Ok(());
            }
            _ => {
                error!("Missing authorization code or state. Aborting.");
                return Ok(());
            }
        };

        self.request_token(&code)?;

        Ok(())
    }

    fn authorize_user(&self) -> () {
        let domain = &self.domain;
        let redirect_uri = &self.redirect_uri;
        let scope = &self.scope;
        let audience = &self.audience;
        let client_id = &self.client_id;
        let code_challenge = &self.code_challenge;
        let state = &self.state;

        let authorize_url = format!(
            "{domain}/authorize?response_type=code\
                 &code_challenge={code_challenge}\
                 &code_challenge_method=S256\
                 &client_id={client_id}&\
                 &redirect_uri={redirect_uri}\
                 &scope={scope}\
                 &audience={audience}\
                 &state={state}"
        );

        info!("Opening browser for login...");
        if let Err(e) = open::that(&authorize_url) {
            warn!("Failed to open browser automatically: {e}");
            info!("Please open this URL manually:\n{authorize_url}");
        }
    }

    fn request_token(&self, code: &str) -> Result<(), io::Error> {
        let domain = &self.domain;
        let redirect_uri = &self.redirect_uri;
        let client_id = &self.client_id;
        let code_verifier = &self.code_verifier;

        let mut token_resp = ureq::post(&format!("{domain}/oauth/token"))
            .content_type("application/x-www-form-urlencoded")
            .send_form(vec![
                ("grant_type", "authorization_code"),
                ("client_id", &client_id),
                ("code_verifier", &code_verifier),
                ("code", &code),
                ("redirect_uri", &redirect_uri),
            ])
            .unwrap();

        let body_content = token_resp.body_mut().read_to_string().unwrap();

        if token_resp.status().is_success() {
            match serde_json::from_str::<TokenSuccess>(&body_content) {
                Ok(success) => {
                    info!("Login successful.");
                    println!("Access Token: {}", success.access_token);
                    if let Some(idt) = success.id_token {
                        println!("ID Token: {}", idt);
                    }
                    if let Some(rt) = success.refresh_token {
                        println!("Refresh Token: {}", rt);
                    }
                    // TODO: Persist tokens securely with your storage.
                }
                Err(e) => {
                    error!("Failed parsing token response: {e}");
                    error!("Raw response: {body_content}");
                }
            }
        } else {
            if let Ok(err) = serde_json::from_str::<TokenError>(&body_content) {
                error!("Token exchange failed: {}", err.error);
                if let Some(desc) = err.error_description {
                    error!("{desc}");
                }
            } else {
                error!("Unexpected token response: {body_content}");
            }
        }

        Ok(())
    }
}

/// Generate PKCE code_verifier and code_challenge and state.
fn generate_secrets() -> (String, String, String) {
    let mut code_verifier_bytes = [0u8; 32];
    OsRng.try_fill_bytes(&mut code_verifier_bytes).unwrap();
    let code_verifier = general_purpose::URL_SAFE_NO_PAD.encode(code_verifier_bytes);

    let code_challenge_bytes = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = general_purpose::URL_SAFE_NO_PAD.encode(code_challenge_bytes);

    let mut state_bytes = [0u8; 16];
    OsRng.try_fill_bytes(&mut state_bytes).unwrap();
    let state = general_purpose::URL_SAFE_NO_PAD.encode(state_bytes);

    (code_verifier, code_challenge, state)
}
