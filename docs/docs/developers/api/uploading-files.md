# Uploading Files

File uploads work by first sending a file to the server and then using the ID provided.

You can find out what kinds of files you can upload by visiting [the API documentation](https://cdn.stoatusercontent.com/scalar).

To upload a file, pick the desired tag then send a **POST** to `{endpoint}/{tag}` along with a `multipart/form-data` body with one field `file` that contains the file you wish to upload.

You must specify session/bot authentication token as with any other API route.

You will receive the following JSON response:

```json
{
  "id": "0"
}
```

You can use the ID wherever a file is required in the API.

Code sample in JavaScript using Fetch API:

```js
const body = new FormData();
body.append("file", file);

const data = await fetch(`${endpoint}/${tag}`, {
  method: "POST",
  body,
  headers: {
    "X-Session-Token": "...", // or X-Bot-Token
  },
}).then((res) => res.json());

// use data.id
```

## Differences from old Autumn

If you are migrating from old Autumn, the following key points are important:

- There are only two paths that serve a unique image, the preview version of it (if available) and the original image.
- You should not specify any query parameters under any circumstance, the preview route will serve the optimal size for the content type.
- Preview routes for banners, emojis, backgrounds, and attachments will redirect to the original file where the file is not an image or the image is animated.
- If you are currently using logic to replace the URL path to start/stop animations, you should use the following templates: (NB. this only applies to avatars and icons)
  - Non-animated file: `/{tag}/{file_id}`
  - Animated file: `/{tag}/{file_id}/{file_name}` or `/{tag}/{file_id}/original` (if name unavailable)
