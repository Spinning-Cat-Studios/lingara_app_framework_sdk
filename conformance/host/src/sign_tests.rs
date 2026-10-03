use serde_json::Value;

use crate::case::{Case, SecretName};
use crate::sign::{USER_AGENT, build, secret, signature};

fn case(sign: &str) -> Case {
    let yaml = format!(
        "id: a.b\nbehaviours: [AK1]\nrequest:\n  operation: app.render\n  body: {{ json: {{ slot: home.side, context: [] }} }}\n{sign}expect: {{ status: 200 }}\n"
    );
    serde_yaml::from_str(&yaml).unwrap()
}

/// ADR 30.9.26al AC10: the signer reproduces the Standard Webhooks upstream
/// vector (its secret re-prefixed `lgr_whsec_`) byte for byte. A built
/// request carries exactly the relay's five headers with the
/// `Lingara-Apps/1` user agent; `webhook-id` equals the body's `id` and is
/// `lgr_msg_` + 32 lowercase hex; two secrets give two space-separated `v1`
/// entries, newest first; and the signed bytes are the sent bytes.
#[test]
fn signs_the_upstream_vector_with_a2s_headers() {
    let upstream = signature("lgr_whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw", "msg_p5jXN8AQM9LWM0D4loKWxJek", 1614265330, br#"{"test": 2432232314}"#);
    assert_eq!(upstream.unwrap(), "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=");

    let built = build(&case("  sign: { secrets: [secondary, primary] }\n"), 1_790_000_000).unwrap();
    let names: Vec<_> = built.headers.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, ["webhook-id", "webhook-timestamp", "webhook-signature", "content-type", "user-agent"]);
    assert_eq!(built.header("user-agent"), Some(USER_AGENT));
    assert_eq!(built.header("content-type"), Some("application/json"));
    assert_eq!(built.header("webhook-timestamp"), Some("1790000000"));

    let id = built.header("webhook-id").unwrap();
    let hex = id.strip_prefix("lgr_msg_").expect("lgr_msg_");
    assert!(hex.len() == 32 && hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')), "{id}");
    let body: Value = serde_json::from_slice(&built.body).unwrap();
    assert_eq!((body["type"].as_str(), body["id"].as_str()), (Some("app.render"), Some(id)));

    let entries: Vec<&str> = built.header("webhook-signature").unwrap().split(' ').collect();
    let expected = |name| signature(&secret(name), id, 1_790_000_000, &built.body).unwrap();
    assert_eq!(entries, [expected(SecretName::Secondary), expected(SecretName::Primary)]);

    let other = build(&case(""), 1_790_000_000).unwrap();
    assert_ne!(other.header("webhook-id"), Some(id), "each request has a fresh id");
}
