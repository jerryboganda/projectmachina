use machina_command_model::{
    ClickPayload, CommandEnvelope, CommandKind, CommandMetadata, CommandPayload, EnginePolicy,
    FidelityProfile, NavigationGotoPayload, SemanticQueryPayload, SessionClosePayload,
    SessionCreatePayload, WaitUntil,
};

fn envelope(kind: CommandKind, payload: CommandPayload) -> CommandEnvelope {
    CommandEnvelope {
        command_id: "command-fixture".to_owned(),
        session_id: "session-fixture".to_owned(),
        context_id: None,
        page_id: None,
        kind,
        payload,
        idempotency_key: None,
        deadline_ms: 1_000,
        required_capabilities: Vec::new(),
        metadata: CommandMetadata {
            correlation_id: "correlation-fixture".to_owned(),
            causation_id: None,
            client: "rust-consumer-fixture".to_owned(),
        },
    }
}

#[test]
fn generated_rust_consumer_uses_discriminated_payloads() {
    let commands = [
        envelope(
            CommandKind::SessionCreateV1,
            CommandPayload::SessionCreate(SessionCreatePayload {
                engine_policy: "prefer-native".to_owned(),
                fidelity_profile: "agent".to_owned(),
            }),
        ),
        envelope(
            CommandKind::NavigationGotoV1,
            CommandPayload::NavigationGoto(NavigationGotoPayload {
                url: "https://one.localhost/navigation".to_owned(),
                wait_until: Some("domcontentloaded".to_owned()),
            }),
        ),
        envelope(
            CommandKind::DomSemanticQueryV1,
            CommandPayload::SemanticQuery(SemanticQueryPayload {
                query: "article".to_owned(),
            }),
        ),
        envelope(
            CommandKind::InteractionClickV1,
            CommandPayload::Click(ClickPayload {
                selector: "#submit".to_owned(),
            }),
        ),
        envelope(
            CommandKind::SessionCloseV1,
            CommandPayload::SessionClose(SessionClosePayload { reason: None }),
        ),
    ];

    assert!(commands.iter().all(CommandEnvelope::payload_matches_kind));
    assert_eq!(commands[0].payload.kind(), CommandKind::SessionCreateV1);
    let CommandPayload::SessionCreate(session) = &commands[0].payload else {
        panic!("session command must carry a session payload");
    };
    assert_eq!(
        session.engine_policy_kind(),
        Some(EnginePolicy::PreferNative)
    );
    assert_eq!(
        session.fidelity_profile_kind(),
        Some(FidelityProfile::Agent)
    );
    let CommandPayload::NavigationGoto(navigation) = &commands[1].payload else {
        panic!("navigation command must carry a navigation payload");
    };
    assert_eq!(
        navigation.wait_until_kind(),
        Some(WaitUntil::Domcontentloaded)
    );

    let mut mismatched = commands[0].clone();
    mismatched.kind = CommandKind::NavigationGotoV1;
    assert!(!mismatched.payload_matches_kind());
}

#[test]
fn generated_rust_consumer_exposes_named_wire_constraints() {
    let _ = EnginePolicy::PreferNative;
    let _ = FidelityProfile::Agent;
    let _ = WaitUntil::Domcontentloaded;
}
