# Memory Jogger Heroku Deployment

To create the Memory Jogger app on Heroku, use this button:

[![Deploy](https://www.herokucdn.com/deploy/button.svg)][deploy]

[deploy]: https://heroku.com/deploy?template=https://github.com/rgardner/memory-jogger

The only config variable you need to set is
`MEMORY_JOGGER_POCKET_CONSUMER_KEY`, which can be obtained by creating an
application in the [Pocket Developer
Portal](https://getpocket.com/developer/apps/):

- Permissions: Retrieve
- Platforms: Desktop (other)

To send emails, see the "Email Setup" section of the [README](../README.md).

Once the app has been created, deploy the code using [Heroku Container
Registry](https://devcenter.heroku.com/articles/container-registry-and-runtime):

```sh
HEROKU_APP_NAME=<YOUR_HEROKU_APP_NAME> invoke deploy
```

Finally, set up a [Heroku
Scheduler](https://devcenter.heroku.com/articles/scheduler) job to
periodically send emails:
https://dashboard.heroku.com/apps/<YOUR_HEROKU_APP_NAME>/scheduler:

- Job: `RUST_BACKTRACE=1 memory_jogger relevant --email --from-email <from_email_address>`
