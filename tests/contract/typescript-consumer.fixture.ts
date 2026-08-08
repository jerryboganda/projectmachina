import {
  CommandEnvelope,
  CommandKind,
  CommandPayloadFor,
  EnginePolicy,
  FidelityProfile,
  SessionCreatePayload,
  WaitUntil
} from "../../packages/contracts-ts/src/command-model.js";

const common = {
  command_id: "command-fixture",
  session_id: "session-fixture",
  deadline_ms: 1_000,
  required_capabilities: [],
  metadata: {
    correlation_id: "correlation-fixture",
    client: "typescript-consumer-fixture"
  }
};

const commands: CommandEnvelope[] = [
  {
    ...common,
    kind: CommandKind.sessionCreateV1,
    payload: {
      engine_policy: EnginePolicy.preferNative,
      fidelity_profile: FidelityProfile.agent
    }
  },
  {
    ...common,
    kind: CommandKind.navigationGotoV1,
    payload: {
      url: "https://one.localhost/navigation",
      wait_until: WaitUntil.domcontentloaded
    }
  },
  {
    ...common,
    kind: CommandKind.domSemanticQueryV1,
    payload: { query: "article" }
  },
  {
    ...common,
    kind: CommandKind.interactionClickV1,
    payload: { selector: "#submit" }
  },
  {
    ...common,
    kind: CommandKind.sessionCloseV1,
    payload: {}
  }
];

function payloadSummary(command: CommandEnvelope): string {
  switch (command.kind) {
    case CommandKind.sessionCreateV1:
      return command.payload.engine_policy;
    case CommandKind.navigationGotoV1:
      return command.payload.url;
    case CommandKind.domSemanticQueryV1:
      return command.payload.query;
    case CommandKind.interactionClickV1:
      return command.payload.selector;
    case CommandKind.sessionCloseV1:
      return command.payload.reason ?? "closed";
  }
}

type Assert<T extends true> = T;
type NavigationPayload = Extract<
  CommandEnvelope,
  { kind: CommandKind.navigationGotoV1 }
>["payload"];
type _NavigationHasUrl = Assert<NavigationPayload extends { url: string } ? true : false>;
type _NavigationRejectsSessionPayload = Assert<
  NavigationPayload extends SessionCreatePayload ? false : true
>;
type _MappedNavigationPayload = Assert<
  CommandPayloadFor<CommandKind.navigationGotoV1> extends NavigationPayload ? true : false
>;

void commands.map(payloadSummary);
