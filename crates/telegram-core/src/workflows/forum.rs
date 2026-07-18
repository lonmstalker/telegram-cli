//! forum workflows.

use super::*;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ForumTopicCursor {
    pub date: i32,
    pub message_id: i64,
    pub topic_id: i32,
}

pub struct ForumTopicsQuery<'query> {
    pub chat_id: i64,
    pub query: &'query str,
    pub count: usize,
    pub page_limit: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ForumTopicsPage {
    pub topics: Vec<Value>,
    pub pages: usize,
    pub total_count: i32,
    pub next_cursor: Option<ForumTopicCursor>,
    pub boundary: PageBoundary,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicMutationOutcome {
    AlreadyApplied,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ForumTopicMutationReceipt {
    pub chat_id: i64,
    pub topic_id: i32,
    pub is_closed: bool,
    pub outcome: TopicMutationOutcome,
    pub complete: bool,
}

pub fn forum_topics(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: ForumTopicsQuery<'_>,
    deadline: Instant,
) -> Result<ForumTopicsPage, ChatWorkflowError> {
    require_resynced(runtime)?;
    forum_topics_with(query, |request| {
        invoke(runtime, policy, "getForumTopics", request, deadline)
    })
}

pub fn set_forum_topic_closed(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    topic_id: i32,
    is_closed: bool,
    deadline: Instant,
) -> Result<ForumTopicMutationReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    set_forum_topic_closed_with(chat_id, topic_id, is_closed, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub(super) fn forum_topics_with(
    query: ForumTopicsQuery<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<ForumTopicsPage, ChatWorkflowError> {
    if query.count == 0 || !(1..=100).contains(&query.page_limit) {
        return Err(ChatWorkflowError::InvalidPageOptions);
    }
    let mut cursor = ForumTopicCursor {
        date: 0,
        message_id: 0,
        topic_id: 0,
    };
    let mut seen_cursors = BTreeSet::from([cursor]);
    let mut seen_topics = BTreeSet::new();
    let mut topics = Vec::new();
    let mut pages = 0;
    loop {
        let response = call(json!({
            "@type":"getForumTopics",
            "chat_id":query.chat_id,
            "query":query.query,
            "offset_date":cursor.date,
            "offset_message_id":cursor.message_id,
            "offset_forum_topic_id":cursor.topic_id,
            "limit":query.page_limit
        }))?;
        pages += 1;
        if response.as_value()["@type"] != "forumTopics" {
            return Err(ChatWorkflowError::UnexpectedResult {
                method: "getForumTopics",
            });
        }
        let total_count = required_i32(response.as_value(), "total_count", "getForumTopics")?;
        let page =
            response.as_value()["topics"]
                .as_array()
                .ok_or(ChatWorkflowError::InvalidResult {
                    method: "getForumTopics",
                    field: "topics",
                })?;
        for topic in page {
            if topic["@type"] != "forumTopic" || topic["info"]["@type"] != "forumTopicInfo" {
                return Err(ChatWorkflowError::InvalidResult {
                    method: "getForumTopics",
                    field: "topics[].@type",
                });
            }
            let topic_id = required_i32(&topic["info"], "forum_topic_id", "getForumTopics")?;
            if seen_topics.insert(topic_id) {
                topics.push(topic.clone());
                if topics.len() == query.count {
                    let next_cursor = forum_topic_cursor(response.as_value())?;
                    return Ok(forum_topics_page(
                        topics,
                        pages,
                        total_count,
                        Some(next_cursor),
                        PageBoundary::Count,
                    ));
                }
            }
        }
        let next_cursor = forum_topic_cursor(response.as_value())?;
        if next_cursor
            == (ForumTopicCursor {
                date: 0,
                message_id: 0,
                topic_id: 0,
            })
        {
            return Ok(forum_topics_page(
                topics,
                pages,
                total_count,
                None,
                PageBoundary::Exhausted,
            ));
        }
        if !seen_cursors.insert(next_cursor) {
            return Ok(forum_topics_page(
                topics,
                pages,
                total_count,
                Some(next_cursor),
                PageBoundary::NoProgress,
            ));
        }
        cursor = next_cursor;
    }
}

fn forum_topic_cursor(value: &Value) -> Result<ForumTopicCursor, ChatWorkflowError> {
    Ok(ForumTopicCursor {
        date: required_i32(value, "next_offset_date", "getForumTopics")?,
        message_id: required_i64(value, "next_offset_message_id", "getForumTopics")?,
        topic_id: required_i32(value, "next_offset_forum_topic_id", "getForumTopics")?,
    })
}

fn forum_topics_page(
    topics: Vec<Value>,
    pages: usize,
    total_count: i32,
    next_cursor: Option<ForumTopicCursor>,
    boundary: PageBoundary,
) -> ForumTopicsPage {
    ForumTopicsPage {
        topics,
        pages,
        total_count,
        next_cursor,
        boundary,
        complete: boundary != PageBoundary::NoProgress,
    }
}

pub(super) fn set_forum_topic_closed_with(
    chat_id: i64,
    topic_id: i32,
    is_closed: bool,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<ForumTopicMutationReceipt, ChatWorkflowError> {
    let request = || json!({"@type":"getForumTopic","chat_id":chat_id,"forum_topic_id":topic_id});
    if call("getForumTopic", request()).and_then(|topic| forum_topic_closed(topic.as_value()))?
        == is_closed
    {
        return Ok(forum_topic_mutation_receipt(
            chat_id,
            topic_id,
            is_closed,
            TopicMutationOutcome::AlreadyApplied,
        ));
    }
    let mutation = call(
        "toggleForumTopicIsClosed",
        json!({
            "@type":"toggleForumTopicIsClosed",
            "chat_id":chat_id,
            "forum_topic_id":topic_id,
            "is_closed":is_closed
        }),
    );
    match mutation {
        Ok(response) => expect_ok(response, "toggleForumTopicIsClosed")?,
        Err(ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))) => {}
        Err(error) => return Err(error),
    }
    let outcome = match call("getForumTopic", request())
        .and_then(|topic| forum_topic_closed(topic.as_value()))
    {
        Ok(actual) if actual == is_closed => TopicMutationOutcome::Verified,
        Ok(_)
        | Err(ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))) => {
            TopicMutationOutcome::Uncertain
        }
        Err(error) => return Err(error),
    };
    Ok(forum_topic_mutation_receipt(
        chat_id, topic_id, is_closed, outcome,
    ))
}

fn forum_topic_closed(value: &Value) -> Result<bool, ChatWorkflowError> {
    if value["@type"] != "forumTopic" || value["info"]["@type"] != "forumTopicInfo" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getForumTopic",
        });
    }
    required_bool(&value["info"], "is_closed", "getForumTopic")
}

fn forum_topic_mutation_receipt(
    chat_id: i64,
    topic_id: i32,
    is_closed: bool,
    outcome: TopicMutationOutcome,
) -> ForumTopicMutationReceipt {
    ForumTopicMutationReceipt {
        chat_id,
        topic_id,
        is_closed,
        outcome,
        complete: outcome != TopicMutationOutcome::Uncertain,
    }
}
