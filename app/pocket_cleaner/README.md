# Pocket Cleaner

![Pocket Cleaner CI](https://github.com/rgardner/pocket-cleaner/workflows/Pocket%20Cleaner%20CI/badge.svg)

Finds items from your Pocket library that are relevant to trending news.

```sh
$ pocket_cleaner
1. amirrajan/survivingtheappstore (Why: Real Madrid)
2. CppCon 2017: Nicolas Guillemot “Design Patterns for Low-Level Real-Time Rendering” (Why: Real Madrid)
3. I Am Legend author Richard Matheson dies (Why: Mikaela Spielberg)
4. Record and share your terminal sessions, the right way. (Why: Mikaela Spielberg)
5. Firefox (1982) (Why: Carrie Symonds)
6. Navy Drone Lands on Aircraft Carrier (Why: Carrie Symonds)
7. Hillary Clinton on the Sanctity of Protecting Classified Information (Why: Drake)
8. EFF’s Game Plan for Ending Global Mass Surveillance (Why: Drake)
9. Drawing with Ants: Generative Art with Ant Colony Optimization Algorithms (Why: El Clasico 2020)
10. All 50+ Adobe apps explained in 10 minutes (Why: El Clasico 2020)
```

## Getting Started

Set the following environment variables:

- `POCKET_CLEANER_CONSUMER_KEY`
  - Create a Pocket app on the [Pocket Developer
    Portal](https://getpocket.com/developer/apps/)
- `POCKET_TEMP_USER_ACCESS_TOKEN`
  - This will go away soon, but for now, manually use the [Pocket Authentication API](https://getpocket.com/developer/docs/authentication) to obtain your user access token.

```sh
export POCKET_CLEANER_CONSUMER_KEY="<YOUR_POCKET_APP_CONSUMER_KEY>"
export POCKET_TEMP_USER_ACCESS_TOKEN="<YOUR_USER_ACCESS_TOKEN>"
```

Then, run `cargo run` to build and run Pocket Cleaner to obtain
items from your Pocket list that are relevant to trending news.

## Contributing

Pocket Cleaner uses [Invoke][pyinvoke] to manage build task execution.

Install Python 3.8+ and [Invoke][pyinvoke].

To use auto-reload functionality, run:

```sh
invoke run --autoreload
```

To run in a Docker container, run:

```sh
invoke run --docker
```

[pyinvoke]: https://www.pyinvoke.org/

### References

- [Google Trends](https://trends.google.com/trends/)
  - [Unofficial JS Reference Client Library](https://github.com/pat310/google-trends-api)
- [Pocket](https://getpocket.com/)
  - [Pocket Developer Homepage](https://getpocket.com/developer/)
  - [Pocket Authentication API](https://getpocket.com/developer/docs/authentication)
  - [Pocket Retrieve API](https://getpocket.com/developer/docs/v3/retrieve)
- [SendGrid](https://sendgrid.com/)
  - [SendGrid v3 Web API](https://sendgrid.com/docs/API_Reference/api_v3.html)
  - [SendGrid Send Mail API](https://sendgrid.com/docs/API_Reference/Web_API_v3/Mail/index.html)
