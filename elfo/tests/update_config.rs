#![allow(missing_docs)]
#![cfg(feature = "test-util")]

use serde::Deserialize;
use toml::toml;

use elfo::{
    config::AnyConfig,
    messages::{ConfigRejected, ConfigUpdated, UpdateConfig},
    prelude::*,
};

#[tokio::test]
async fn singleton_actor_update_config() {
    #[message]
    struct StartSingleton;

    #[message(ret = usize)]
    struct GetLimit;

    #[derive(Debug, Clone, Deserialize)]
    struct Config {
        limit: usize,
    }

    let blueprint = ActorGroup::new()
        .config::<Config>()
        .exec(move |mut ctx| async move {
            while let Some(envelope) = ctx.recv().await {
                msg!(match envelope {
                    StartSingleton | ConfigUpdated => continue,
                    (GetLimit, token) => {
                        ctx.respond(token, ctx.config().limit);
                    }
                    _ => unreachable!(),
                });
            }
        });

    let config = toml! {
        limit = 128
    };
    let proxy = elfo::test::proxy(blueprint, config).await;

    proxy.send(StartSingleton).await;
    assert_eq!(proxy.request(GetLimit).await, 128);

    // Update with valid config through message.
    let config = AnyConfig::deserialize(toml! {
        limit = 256
    })
    .unwrap();
    proxy.send(UpdateConfig::new(config)).await;
    assert_eq!(proxy.request(GetLimit).await, 256);

    // Update with valid config through request.
    let config = AnyConfig::deserialize(toml! {
        limit = 512
    })
    .unwrap();
    assert!(proxy.request(UpdateConfig::new(config)).await.is_ok());
    assert_eq!(proxy.request(GetLimit).await, 512);

    // Update with invalid config through message.
    let config = AnyConfig::deserialize(toml! {
        limit = -256
    })
    .unwrap();
    proxy.send(UpdateConfig::new(config.clone())).await;
    assert_eq!(proxy.request(GetLimit).await, 512);

    // Update with invalid config through request.
    assert!(matches!(
        proxy.request(UpdateConfig::new(config)).await,
        Err(ConfigRejected { .. })
    ));
    assert_eq!(proxy.request(GetLimit).await, 512);
}

#[tokio::test]
#[should_panic(expected = "subject:\n- panic: intentional panic")]
async fn panic_in_deserialize() {
    #[derive(Debug, Clone)]
    struct BadConfig;

    impl<'de> Deserialize<'de> for BadConfig {
        fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::de::Deserializer<'de>,
        {
            panic!("intentional panic");
        }
    }

    let blueprint = ActorGroup::new()
        .config::<BadConfig>()
        .exec(|_| async { unreachable!() });

    let _proxy = elfo::test::proxy(blueprint, AnyConfig::default()).await;
}
