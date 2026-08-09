import { HttpTransport, MachinaClient } from "@machina/sdk-typescript";

const client = new MachinaClient(new HttpTransport("http://127.0.0.1:8080"));
const session = await client.createSession();
try {
  await session.navigate("https://fixture.local/");
  const result = await session.page().extract("main article");
  console.log(result.result);
} finally {
  await session.close();
}
