use std::collections::HashMap;

use aether_common::db::BroadcastMessage;

use crate::db::SubscriptionOptions;

pub async fn compute() {}

fn should_send_message(
    client_id: &String,
    message: &BroadcastMessage,
    subscriptions: &HashMap<String, SubscriptionOptions>,
) -> bool {
    // This is some nasty logic
    // If the channel is `global`, just send the message
    // Otherwise, check if you're subscribed
    // If you're subscribed and it's not from you, send it
    // If you're subscribed and it is from you, check to see if you subscribed to self messages
    // If you're not subscribed and the channel is not `global`, ignore it
    message.channel == "global"
        || (subscriptions.contains_key(&message.channel)
            && (message.client_id != *client_id
                || *subscriptions
                    .get(&message.channel)
                    .map(|sub| &sub.subscribe_to_self)
                    .unwrap_or(&false)))
}
