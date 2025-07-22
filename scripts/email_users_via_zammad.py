# requires httpx off pypi
import httpx
import math
from os import getenv
import asyncio

URL = getenv("ZAMMAD_URL")
TOKEN = getenv("ZAMMAD_TOKEN")
EMAIL_FILE = getenv("EMAILS_FILE")
PAYLOAD_TEMPLATE = getenv("PAYLOAD_FILE")

assert URL and TOKEN and EMAIL_FILE and PAYLOAD_TEMPLATE, ValueError("Expected env variables EMAIL_FILE, PAYLOAD_FILE, ZAMMAD_URL and ZAMMAD_TOKEN.")

URL = URL.strip("/") + "/api/v1/tickets"

async def worker(payload: dict, emails: list[str]) -> None:
    async with httpx.AsyncClient(headers={"Authorization": f"Token token={TOKEN}"}) as client:
        assert URL # redundant but makes pyright happy

        for email in emails:
            payload["customer_id"] = f"guess:{email}"
            payload["article"]["to"] = email
            print(email)
            resp = await client.post(URL, json=payload)
            if resp.status_code > 300:
                raise RuntimeError(f"Failed to create ticket: {resp.status_code}\n{resp.read()}")

async def main():
    assert EMAIL_FILE and PAYLOAD_TEMPLATE # pyright happy time

    with open(EMAIL_FILE) as f:
        emails = [x.strip() for x in f.read().split("\n")]
    
    with open(PAYLOAD_TEMPLATE) as f:
        p = {}
        exec(f.read(), p)
        payload = p["PAYLOAD"]

    # WORKER_COUNT = min(max(round(len(emails) / 10), 1), 10)
    # EMAILS_PER = math.ceil(len(emails) / WORKER_COUNT)
    # print(WORKER_COUNT, EMAILS_PER)
    # tasks = []

    # for worker_id in range(WORKER_COUNT):
    #     worker_emails = emails[EMAILS_PER * worker_id:EMAILS_PER * worker_id + 1]
    #     tasks.append(asyncio.create_task(worker(payload, worker_emails)))

    # await asyncio.gather(*tasks)

    await worker(payload, emails)

asyncio.run(main())