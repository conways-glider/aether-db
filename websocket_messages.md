# Websocket Messages for Testing

## Broadcast Message Command

```
{"send_broadcast": {"channel":"global","message":"test"}}
{"send_broadcast": {"channel":"testing","message":"test"}}
```

## Subscribe Message Command

```
{"subscribe_broadcast": {"channel":"testing"}}
```

## Unsubscribe Message Command

```
{"unsubscribe_broadcast": "testing"}
```

## String Database

```
{"set_string": {"key":"test","value":"value"}}
{"get_string": {"key":"test"}}

```
