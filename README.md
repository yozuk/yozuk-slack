# yozuk-slack
Yozuk slack bot

## Slack Bot Configuration

### Scopes

`yozuk-slack` requires the following permission scopes.

- `app_mentions:read`
- `chat:write`
- `im:history`

### Event Subscriptions

`yozuk-slack` receives requests via the [Slack Events API](https://api.slack.com/apis/connections/events-api).

You have to add the following bot user events.

- `app_mention`
- `message.im`