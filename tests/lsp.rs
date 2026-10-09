use indoc::indoc;
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CodeActionOrCommand, InitializeResult, NumberOrString, PublishDiagnosticsParams, TextEdit,
};
use mdlint::formatter;
use mdlint::server::run_server_with_connection;
use std::thread;

// ── helpers ───────────────────────────────────────────────────────────────────

fn next_message(conn: &Connection) -> Message {
    conn.receiver.recv().expect("expected a message")
}

fn next_response(conn: &Connection) -> Response {
    match next_message(conn) {
        Message::Response(r) => r,
        other => panic!("expected Response, got {other:?}"),
    }
}

fn next_notification(conn: &Connection) -> Notification {
    match next_message(conn) {
        Message::Notification(n) => n,
        other => panic!("expected Notification, got {other:?}"),
    }
}

fn send_request(conn: &Connection, id: i32, method: &str, params: serde_json::Value) {
    conn.sender
        .send(Message::Request(Request {
            id: RequestId::from(id),
            method: method.to_owned(),
            params,
        }))
        .unwrap();
}

fn send_notification(conn: &Connection, method: &str, params: serde_json::Value) {
    conn.sender
        .send(Message::Notification(Notification {
            method: method.to_owned(),
            params,
        }))
        .unwrap();
}

/// Perform the LSP initialize handshake from the client side.
fn initialize(client: &Connection) {
    send_request(
        client,
        1,
        "initialize",
        serde_json::json!({
            "processId": null,
            "capabilities": {},
            "rootUri": null
        }),
    );

    let resp = next_response(client);
    let result: InitializeResult = match resp.response_result {
        Ok(result) => serde_json::from_value(result).unwrap(),
        Err(error) => panic!("initialize error: {error:?}"),
    };
    assert!(result.capabilities.text_document_sync.is_some());

    send_notification(client, "initialized", serde_json::json!({}));
}

fn shutdown(client: &Connection) {
    send_request(client, 999, "shutdown", serde_json::json!(null));
    let resp = next_response(client);
    assert!(
        resp.response_result.is_ok(),
        "shutdown error: {:?}",
        resp.response_result
    );
    send_notification(client, "exit", serde_json::json!(null));
}

#[test]
fn invalid_configuration_is_reported_and_can_be_repaired() {
    let dir = tempfile::TempDir::new().unwrap();
    let config = dir.path().join("mdlint.toml");
    std::fs::write(&config, "default_enabled = [").unwrap();
    let uri = url::Url::from_file_path(dir.path().join("doc.md"))
        .unwrap()
        .to_string();
    let (server_conn, client) = Connection::memory();
    let server_thread =
        thread::spawn(move || run_server_with_connection(&server_conn, None).unwrap());
    initialize(&client);
    send_notification(
        &client,
        "textDocument/didOpen",
        serde_json::json!({"textDocument": {
            "uri": uri, "languageId": "markdown", "version": 1, "text": "é   \n"
        }}),
    );
    let notification = next_notification(&client);
    assert_eq!(notification.method, "window/showMessage");
    let error: lsp_types::ShowMessageParams = serde_json::from_value(notification.params).unwrap();
    assert_eq!(error.typ, lsp_types::MessageType::ERROR);
    assert!(error.message.contains("Configuration error"));
    assert!(error.message.contains(&uri));
    let notification = next_notification(&client);
    assert_eq!(notification.method, "textDocument/publishDiagnostics");
    let diagnostics: PublishDiagnosticsParams =
        serde_json::from_value(notification.params).unwrap();
    assert!(diagnostics.diagnostics.is_empty());

    let action_params = serde_json::json!({
        "textDocument": {"uri": uri},
        "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
        "context": {"diagnostics": []}
    });
    send_request(&client, 2, "textDocument/codeAction", action_params.clone());
    let error = next_response(&client).response_result.unwrap_err();
    assert_eq!(error.code, -32603);
    assert!(error.message.contains("Configuration error"));

    std::fs::write(
        &config,
        "default_enabled = false\n[rules.MD009]\nenabled = true\n",
    )
    .unwrap();
    send_notification(
        &client,
        "textDocument/didChange",
        serde_json::json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": "é   \n"}]
        }),
    );
    let notification = next_notification(&client);
    assert_eq!(notification.method, "textDocument/publishDiagnostics");
    let diagnostics: PublishDiagnosticsParams =
        serde_json::from_value(notification.params).unwrap();
    assert_eq!(diagnostics.diagnostics.len(), 1);
    assert_eq!(
        diagnostics.diagnostics[0].code,
        Some(NumberOrString::String("MD009".to_owned()))
    );
    send_request(&client, 3, "textDocument/codeAction", action_params);
    let actions: Vec<CodeActionOrCommand> =
        serde_json::from_value(next_response(&client).response_result.unwrap()).unwrap();
    assert_eq!(actions.len(), 1);
    assert!(
        matches!(&actions[0], CodeActionOrCommand::CodeAction(action) if action.title == "Fix MD009")
    );
    shutdown(&client);
    server_thread.join().unwrap();
}

// ── test ──────────────────────────────────────────────────────────────────────

#[test]
fn lsp_full_lifecycle() {
    let (server_conn, client_conn) = Connection::memory();

    let server_thread =
        thread::spawn(move || run_server_with_connection(&server_conn, None).unwrap());

    // 1. Initialize handshake
    initialize(&client_conn);

    // MD022 violation: no blank line between headings.
    let content = indoc! {"
        # Title
        ## Section
    "};

    // 2. didOpen → publishDiagnostics
    send_notification(
        &client_conn,
        "textDocument/didOpen",
        serde_json::json!({
            "textDocument": {
                "uri": "file:///tmp/test.md",
                "languageId": "markdown",
                "version": 1,
                "text": content
            }
        }),
    );

    let notif = next_notification(&client_conn);
    assert_eq!(notif.method, "textDocument/publishDiagnostics");
    let params: PublishDiagnosticsParams =
        serde_json::from_value(notif.params).expect("parse publishDiagnostics");
    assert!(
        !params.diagnostics.is_empty(),
        "expected at least one diagnostic"
    );
    // Verify at least one diagnostic has an MD rule code.
    assert!(
        params.diagnostics.iter().any(|d| {
            matches!(&d.code, Some(NumberOrString::String(code)) if code.starts_with("MD"))
        }),
        "expected diagnostic with MD rule code"
    );
    assert!(
        params
            .diagnostics
            .iter()
            .all(|d| d.severity == Some(lsp_types::DiagnosticSeverity::WARNING)),
        "all diagnostics should be warnings"
    );

    // 3. formatting → TextEdit
    send_request(
        &client_conn,
        2,
        "textDocument/formatting",
        serde_json::json!({
            "textDocument": { "uri": "file:///tmp/test.md" },
            "options": { "tabSize": 2, "insertSpaces": true }
        }),
    );

    let resp = next_response(&client_conn);
    let edits: Vec<TextEdit> = match resp.response_result {
        Ok(result) => serde_json::from_value(result).unwrap(),
        Err(error) => panic!("formatting error: {error:?}"),
    };
    // Content needs formatting; expect exactly one whole-doc edit.
    assert_eq!(edits.len(), 1, "expected one TextEdit");
    assert_eq!(
        edits[0].new_text,
        formatter::format(content),
        "TextEdit new_text must equal formatter::format(content)"
    );

    // 4. codeAction for the line of any fixable violation
    let fixable_line = params
        .diagnostics
        .iter()
        .map(|d| d.range.start.line)
        .next()
        .unwrap_or(0);

    send_request(
        &client_conn,
        3,
        "textDocument/codeAction",
        serde_json::json!({
            "textDocument": { "uri": "file:///tmp/test.md" },
            "range": {
                "start": { "line": fixable_line, "character": 0 },
                "end":   { "line": fixable_line, "character": 0 }
            },
            "context": { "diagnostics": [] }
        }),
    );

    let resp = next_response(&client_conn);
    // Must parse as a valid array of CodeActionOrCommand.
    let _actions: Vec<CodeActionOrCommand> = match resp.response_result {
        Ok(result) => serde_json::from_value(result).expect("parse codeAction result"),
        Err(error) => panic!("codeAction error: {error:?}"),
    };

    // 5. shutdown + exit → server thread completes without panic
    shutdown(&client_conn);

    server_thread.join().expect("server thread panicked");
}
