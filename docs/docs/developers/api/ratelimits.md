# Rate Limits

Stoat uses a fixed-window ratelimiting algorithm:

- You are given a set amount of calls per each named bucket.
- Any calls past this limit will result in 429 errors.
- Buckets are replenished after 10 seconds from initial request.

## Buckets

There are distinct buckets that you may be calling against, none of these affect each other and can be used up independently of one another.

|   Method | Path                        | Limit |
| -------: | --------------------------- | :---: |
|          | `/users`                    |  20   |
|  `PATCH` | `/users/:id`                |   2   |
|          | `/users/:id/default_avatar` |  255  |
|          | `/bots`                     |  10   |
|          | `/channels`                 |  15   |
|   `POST` | `/channels/:id/messages`    |  10   |
|          | `/servers`                  |   5   |
|          | `/auth`                     |   3   |
| `DELETE` | `/auth`                     |  255  |
|          | `/safety`                   |  15   |
|          | `/safety/report`            |   3   |
|          | `/swagger`                  |  100  |
|          | `/*`                        |  20   |

## Headers

There are multiple headers you can use to figure out when you can and cannot send requests, and to determine when you can next send a request.

| Header                    |   Type   | Description                                      |
| ------------------------- | :------: | ------------------------------------------------ |
| `X-RateLimit-Limit`       | `number` | Maximum number of calls allowed for this bucket. |
| `X-RateLimit-Bucket`      | `string` | Unique identifier for this bucket.               |
| `X-RateLimit-Remaining`   | `number` | Remaining number of calls left for this bucket.  |
| `X-RateLimit-Reset-After` | `number` | Milliseconds left until calls are replenished.   |

## Rate Limited Response

When you receive `429 Too Many Requests`, you will also receive a JSON body with the schema:

```typescript
interface Response {
  // Milliseconds until calls are replenished
  retry_after: number;
}
```
