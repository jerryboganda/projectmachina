import asyncio

from machina_sdk import HttpTransport, MachinaClient


async def main() -> None:
    client = MachinaClient(HttpTransport("http://127.0.0.1:8080"))
    session = await client.create_session()
    try:
        await session.navigate("https://fixture.local/")
        result = await session.page().extract("main article")
        print(result.result)
    finally:
        await session.close()


if __name__ == "__main__":
    asyncio.run(main())
