# Pocket Cleaner

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
