# Websocket Messages for Testing

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

## String Database

```json
{"set_string": {"key":"test", "value":"value"}}
{"set_string": {"key":"expire", "value":"value", "expiration": 20}}
{"get_string": {"key":"test"}}
{"get_string": {"key":"expire"}}
```

## JSON Database

```json
{"set_json": {"key":"test", "value":"value"}}
{"set_json": {"key":"test_object", "value":{ "nested_value": "value" }}}
{"set_json": {"key":"bad_json", "value":{ "nested_value": "value" }}
{"set_json": {"key":"expire", "value":"value", "expiration": 20}}
{"get_json": {"key":"test"}}
{"get_json": {"key":"test_object"}}
{"get_json": {"key":"expire"}}
```
