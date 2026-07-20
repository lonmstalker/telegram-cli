use serde_json::{Value, json};

use super::{ChatList, ChatWorkflowError, load_until_terminal};
use crate::registry::TdObject;

fn workflow_object(value: Value) -> Result<TdObject, ChatWorkflowError> {
    Ok(TdObject::from_value(value).unwrap())
}

#[test]
fn folder_chat_list_uses_exact_target_and_only_404_is_terminal() {
    let expected = json!({
        "@type":"loadChats",
        "chat_list":{"@type":"chatListFolder","chat_folder_id":17},
        "limit":100
    });
    let mut requests = Vec::new();
    let mut responses = [
        json!({"@type":"ok"}),
        json!({"@type":"error","code":404,"message":"Not Found"}),
    ]
    .into_iter();

    let load_calls = load_until_terminal(ChatList::Folder(17).tdjson(), 100, |request| {
        requests.push(request);
        workflow_object(responses.next().unwrap())
    })
    .unwrap();

    assert_eq!(load_calls, 2);
    assert_eq!(requests, [expected.clone(), expected]);
    assert!(matches!(
        load_until_terminal(ChatList::Folder(17).tdjson(), 100, |_| {
            workflow_object(json!({
                "@type":"error",
                "code":400,
                "message":"Chat folder not found"
            }))
        }),
        Err(ChatWorkflowError::Tdlib {
            method: "loadChats",
            code: Some(400)
        })
    ));
}
