# Momento DynamoDB Interceptor
This crate offers an extension for the AWS SDK for DynamoDB that
proxies requests through [Momento](https://gomomento.com).

You create a cache, then configure this interceptor, and your existing
code that talks to DynamoDB through the AWS SDK continues to work as
before. It's just hopefully a little quicker, cheaper, and without
hot key troubles.

All reads serviced from this interceptor are potentially as stale as the
TTL you configure. So if you set a ttl of 60 seconds, you might get 60
second stale responses from DynamoDB for a GetItem.
