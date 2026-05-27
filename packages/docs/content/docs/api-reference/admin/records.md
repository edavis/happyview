---
title: "Records"
---

Browse and manage indexed records. All endpoints require the appropriate `records:*` permission.

```sh tab="cURL" tab-group="language"
# All examples assume $TOKEN is an API key (hv_...)
AUTH="Authorization: Bearer $TOKEN"
```

## List records

```
GET /admin/records
```

Paginated list of records in a collection, ordered by `indexed_at` descending.

| Param        | Type   | Required | Description                               |
| ------------ | ------ | -------- | ----------------------------------------- |
| `collection` | string | yes      | Collection NSID to list records from      |
| `limit`      | number | no       | Max results per page (default 20, max 100)|
| `cursor`     | string | no       | Pagination cursor from a previous response|

```sh tab="cURL" tab-group="language"
curl "http://127.0.0.1:3000/admin/records?collection=xyz.statusphere.status&limit=10" -H "$AUTH"
```

**Response**: `200 OK`

```json
{
  "records": [
    {
      "uri": "at://did:plc:abc/xyz.statusphere.status/3k...",
      "did": "did:plc:abc",
      "collection": "xyz.statusphere.status",
      "rkey": "3k...",
      "cid": "bafyrei...",
      "indexed_at": "2025-01-01T00:00:00Z",
      "record": { "...": "..." },
      "labels": []
    }
  ],
  "cursor": "20"
}
```

`cursor` is omitted when there are no more results.

## List collections

```
GET /admin/records/collections
```

Returns the list of collection NSIDs from registered record-type lexicons. This is a fast lookup (no record counting).

```sh tab="cURL" tab-group="language"
curl http://127.0.0.1:3000/admin/records/collections -H "$AUTH"
```

**Response**: `200 OK`

```json
{
  "collections": [
    "xyz.statusphere.status",
    "app.bsky.feed.post"
  ]
}
```

## Delete a record

```
DELETE /admin/records
```

Delete a single record by AT URI.

| Param | Type   | Required | Description                          |
| ----- | ------ | -------- | ------------------------------------ |
| `uri` | string | yes      | AT URI of the record to delete       |

```sh tab="cURL" tab-group="language"
curl -X DELETE "http://127.0.0.1:3000/admin/records?uri=at://did:plc:abc/xyz.statusphere.status/3k..." -H "$AUTH"
```

**Response**: `204 No Content`

Returns `404` if the record is not found.

## Delete all records in a collection

```
DELETE /admin/records/collection
```

Delete all indexed records for a given collection. Requires both `records:delete` and `records:delete-collection` permissions.

| Param        | Type   | Required | Description                          |
| ------------ | ------ | -------- | ------------------------------------ |
| `collection` | string | yes      | Collection NSID to delete from       |

```sh tab="cURL" tab-group="language"
curl -X DELETE "http://127.0.0.1:3000/admin/records/collection?collection=xyz.statusphere.status" -H "$AUTH"
```

**Response**: `200 OK`

```json
{
  "deleted": 42
}
```
