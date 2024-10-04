# Websocket Messages for Testing

This document details the socket messages that may be sent for testing purposes.

## Broadcast Message Command

```json
{"send_broadcast": {"channel":"global","message":"test"}}
{"send_broadcast": {"channel":"testing","message":"test"}}
```

## Subscribe Message Command

```json
{"subscribe_broadcast": {"channel":"testing"}}
```

## Unsubscribe Message Command

```json
{"unsubscribe_broadcast": "testing"}
```

## Database

### Set

```json
{"set": {"key":"test", "value":{ "data": {"string": "test"}}}}
{"set": {"key":"test", "value":{ "data": {"json": { "test_key": "test_value"}}}}}
{"set": {"key":"test", "value":{ "data": {"int": 1}}}}
```

### Get

```json
{"get": {"key":"test"}}
```
