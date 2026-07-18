//! stars workflows.

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct StarAmountView {
    pub star_count: i64,
    pub nanostar_count: i32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StarBalanceSnapshot {
    pub owner_user_id: i64,
    pub amount: StarAmountView,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StarInvoicePlan {
    pub payment_form_id: i64,
    pub seller_user_id: i64,
    pub star_count: i64,
    pub available_star_count: i64,
    pub risk: RiskClass,
    pub retry: RetryClass,
    pub plan_hash: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StarPaymentOutcome {
    Confirmed,
    VerificationRequired,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StarPaymentReceipt {
    pub payment_form_id: i64,
    pub seller_user_id: i64,
    pub star_count: i64,
    pub outcome: StarPaymentOutcome,
    pub sensitive_data_redacted: bool,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

pub fn star_balance(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    deadline: Instant,
) -> Result<StarBalanceSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    star_balance_with(|method, request| invoke(runtime, policy, method, request, deadline))
}

pub fn plan_star_invoice_payment(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    invoice_name: &str,
    deadline: Instant,
) -> Result<StarInvoicePlan, ChatWorkflowError> {
    require_resynced(runtime)?;
    plan_star_invoice_payment_with(invoice_name, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub fn star_invoice_payment_request(
    invoice_name: &str,
    payment_form_id: i64,
) -> Result<Value, ChatWorkflowError> {
    validate_star_invoice(invoice_name)?;
    if payment_form_id <= 0 {
        return Err(ChatWorkflowError::InvalidPaymentInput);
    }
    Ok(json!({
        "@type":"sendPaymentForm",
        "input_invoice":{"@type":"inputInvoiceName","name":invoice_name},
        "payment_form_id":payment_form_id,
        "order_info_id":"",
        "shipping_option_id":"",
        "credentials":null,
        "tip_amount":0
    }))
}

pub fn apply_star_invoice_payment(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    plan: &StarInvoicePlan,
    invoice_name: &str,
    deadline: Instant,
) -> Result<StarPaymentReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    apply_star_invoice_payment_with(plan, invoice_name, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

struct StarLedger {
    balance: StarBalanceSnapshot,
    transactions: Vec<Value>,
}

fn star_balance_with(
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StarBalanceSnapshot, ChatWorkflowError> {
    Ok(fetch_star_ledger(&mut call)?.balance)
}

pub(super) fn plan_star_invoice_payment_with(
    invoice_name: &str,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StarInvoicePlan, ChatWorkflowError> {
    validate_star_invoice(invoice_name)?;
    let form = call(
        "getPaymentForm",
        json!({
            "@type":"getPaymentForm",
            "input_invoice":{"@type":"inputInvoiceName","name":invoice_name},
            "theme":null
        }),
    )?;
    let (payment_form_id, seller_user_id, star_count) = star_payment_form(form.as_value())?;
    let balance = fetch_star_ledger(&mut call)?.balance.amount.star_count;
    if balance < star_count {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "sufficient_star_balance",
        });
    }
    let request =
        ValidatedRequest::from_value(star_invoice_payment_request(invoice_name, payment_form_id)?)
            .map_err(|_| ChatWorkflowError::InvalidPaymentInput)?;
    let preview =
        PlanPreview::for_request(&request).map_err(|_| ChatWorkflowError::InvalidPaymentInput)?;
    Ok(StarInvoicePlan {
        payment_form_id,
        seller_user_id,
        star_count,
        available_star_count: balance,
        risk: preview.risk,
        retry: preview.retry,
        plan_hash: preview.hash.to_hex(),
    })
}

pub(super) fn apply_star_invoice_payment_with(
    plan: &StarInvoicePlan,
    invoice_name: &str,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StarPaymentReceipt, ChatWorkflowError> {
    let request = star_invoice_payment_request(invoice_name, plan.payment_form_id)?;
    let preview = PlanPreview::for_request(
        &ValidatedRequest::from_value(request.clone())
            .map_err(|_| ChatWorkflowError::InvalidPaymentInput)?,
    )
    .map_err(|_| ChatWorkflowError::InvalidPaymentInput)?;
    if preview.hash.to_hex() != plan.plan_hash
        || preview.risk != plan.risk
        || preview.retry != plan.retry
    {
        return Err(ChatWorkflowError::PlanStale);
    }
    let before = fetch_star_ledger(&mut call)?;
    if before.balance.amount.star_count < plan.star_count {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "sufficient_star_balance",
        });
    }
    match call("sendPaymentForm", request) {
        Ok(response) => {
            if response.as_value()["@type"] != "paymentResult" {
                return Err(ChatWorkflowError::UnexpectedResult {
                    method: "sendPaymentForm",
                });
            }
            if !required_bool(response.as_value(), "success", "sendPaymentForm")? {
                return Ok(star_payment_receipt(
                    plan,
                    if required_string(response.as_value(), "verification_url", "sendPaymentForm")?
                        .is_empty()
                    {
                        StarPaymentOutcome::Uncertain
                    } else {
                        StarPaymentOutcome::VerificationRequired
                    },
                ));
            }
        }
        Err(error) if response_timed_out(&error) => {}
        Err(error) => return Err(error),
    }
    let outcome = match fetch_star_ledger(&mut call) {
        Ok(after)
            if matches!(
                has_new_matching_star_payment(&before, &after, plan),
                Ok(true)
            ) =>
        {
            StarPaymentOutcome::Confirmed
        }
        Ok(_) | Err(_) => StarPaymentOutcome::Uncertain,
    };
    Ok(star_payment_receipt(plan, outcome))
}

fn fetch_star_ledger(
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StarLedger, ChatWorkflowError> {
    let me = call("getMe", json!({"@type":"getMe"}))?;
    if me.as_value()["@type"] != "user" {
        return Err(ChatWorkflowError::UnexpectedResult { method: "getMe" });
    }
    let owner_user_id = required_i64(me.as_value(), "id", "getMe")?;
    let response = call(
        "getStarTransactions",
        json!({
            "@type":"getStarTransactions",
            "owner_id":{"@type":"messageSenderUser","user_id":owner_user_id},
            "subscription_id":"",
            "direction":null,
            "offset":"",
            "limit":100
        }),
    )?;
    let value = response.as_value();
    if value["@type"] != "starTransactions" || value["star_amount"]["@type"] != "starAmount" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getStarTransactions",
        });
    }
    let transactions = value["transactions"]
        .as_array()
        .ok_or(ChatWorkflowError::InvalidResult {
            method: "getStarTransactions",
            field: "transactions",
        })?
        .to_owned();
    for transaction in &transactions {
        if transaction["@type"] != "starTransaction"
            || required_string(transaction, "id", "getStarTransactions")?.is_empty()
        {
            return Err(ChatWorkflowError::UnexpectedResult {
                method: "getStarTransactions",
            });
        }
    }
    Ok(StarLedger {
        balance: StarBalanceSnapshot {
            owner_user_id,
            amount: StarAmountView {
                star_count: required_i64(
                    &value["star_amount"],
                    "star_count",
                    "getStarTransactions",
                )?,
                nanostar_count: required_i32(
                    &value["star_amount"],
                    "nanostar_count",
                    "getStarTransactions",
                )?,
            },
            observed_at: SystemTime::now(),
            freshness: Freshness::ServerSnapshot,
        },
        transactions,
    })
}

fn star_payment_form(value: &Value) -> Result<(i64, i64, i64), ChatWorkflowError> {
    if value["@type"] != "paymentForm" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getPaymentForm",
        });
    }
    let star_count = match value["type"]["@type"].as_str() {
        Some("paymentFormTypeStars") => {
            required_i64(&value["type"], "star_count", "getPaymentForm")?
        }
        Some("paymentFormTypeStarSubscription") => {
            required_i64(&value["type"]["pricing"], "star_count", "getPaymentForm")?
        }
        _ => {
            return Err(ChatWorkflowError::CapabilityDenied {
                capability: "stars_only_payment",
            });
        }
    };
    let payment_form_id = required_i64(value, "id", "getPaymentForm")?;
    let seller_user_id = required_i64(value, "seller_bot_user_id", "getPaymentForm")?;
    if payment_form_id <= 0 || seller_user_id <= 0 || star_count <= 0 {
        return Err(ChatWorkflowError::InvalidPaymentInput);
    }
    Ok((payment_form_id, seller_user_id, star_count))
}

fn has_new_matching_star_payment(
    before: &StarLedger,
    after: &StarLedger,
    plan: &StarInvoicePlan,
) -> Result<bool, ChatWorkflowError> {
    if before.balance.owner_user_id != after.balance.owner_user_id {
        return Ok(false);
    }
    let known = before
        .transactions
        .iter()
        .map(|transaction| required_string(transaction, "id", "getStarTransactions"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    for transaction in &after.transactions {
        let kind = transaction["type"]["@type"].as_str();
        if !known.contains(required_string(transaction, "id", "getStarTransactions")?)
            && matches!(
                kind,
                Some(
                    "starTransactionTypeBotInvoicePurchase"
                        | "starTransactionTypeBotSubscriptionPurchase"
                )
            )
            && required_i64(&transaction["type"], "user_id", "getStarTransactions")?
                == plan.seller_user_id
            && required_i64(
                &transaction["star_amount"],
                "star_count",
                "getStarTransactions",
            )? == -plan.star_count
            && required_i32(
                &transaction["star_amount"],
                "nanostar_count",
                "getStarTransactions",
            )? == 0
            && !required_bool(transaction, "is_refund", "getStarTransactions")?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn star_payment_receipt(plan: &StarInvoicePlan, outcome: StarPaymentOutcome) -> StarPaymentReceipt {
    StarPaymentReceipt {
        payment_form_id: plan.payment_form_id,
        seller_user_id: plan.seller_user_id,
        star_count: plan.star_count,
        outcome,
        sensitive_data_redacted: true,
        complete: outcome == StarPaymentOutcome::Confirmed,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    }
}

fn validate_star_invoice(invoice_name: &str) -> Result<(), ChatWorkflowError> {
    if invoice_name.is_empty()
        || invoice_name.len() > 512
        || invoice_name.chars().any(char::is_control)
    {
        return Err(ChatWorkflowError::InvalidPaymentInput);
    }
    Ok(())
}
