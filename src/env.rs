use std::collections::HashMap;

/// Minimal env vars that we will commonly need across all applications
#[derive(Clone)]
pub enum EnvVar {
    /// DATABASE_URL
    DatabaseUrlSelf,

    /// DATABASE_URL_CORE
    DatabaseUrlCore,

    /// APPLICATION_ID
    ApplicationID,

    /// APPLICATION_SECRET
    ApplicationSecret,

    /// APPLICATION_NAME
    ApplicationName,

    /// APPLICATION_HOST
    ApplicationHost,

    /// This is a catch all. It has no direct map.
    Custom(&'static str),
}

impl From<EnvVar> for &'static str {
    fn from(var: EnvVar) -> Self {
        match var {
            EnvVar::DatabaseUrlSelf => "DATABASE_URL",
            EnvVar::DatabaseUrlCore => "DATABASE_URL_CORE",
            EnvVar::ApplicationID => "APPLICATION_ID",
            EnvVar::ApplicationSecret => "APPLICATION_SECRET",
            EnvVar::ApplicationHost => "APPLICATION_HOST",
            EnvVar::ApplicationName => "APPLICATION_NAME",
            EnvVar::Custom(key) => key, // just pass through the setting
        }
    }
}

impl From<&'static str> for EnvVar {
    fn from(src: &'static str) -> Self {
        match src {
            "DATABASE_URL" => EnvVar::DatabaseUrlSelf,
            "DATABASE_URL_CORE" => EnvVar::DatabaseUrlCore,
            "APPLICATION_ID" => EnvVar::ApplicationID,
            "APPLICATION_SECRET" => EnvVar::DatabaseUrlSelf,
            "APPLICATION_HOST" => EnvVar::ApplicationHost,
            "APPLICATION_NAME" => EnvVar::ApplicationName,
            data => EnvVar::Custom(data),
        }
    }
}

impl std::fmt::Display for EnvVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r: &'static str = self.clone().into();
        write!(f, "{r}")
    }
}

/// fetches a application variable from the .env file or targeted system environment variables
pub fn get(env_var: EnvVar) -> String {
    std::env::var::<&'static str>(env_var.into()).unwrap_or_default()
}

/// checks if a environment variable is set
pub fn exists(env_var: EnvVar) -> bool {
    std::env::var::<&'static str>(env_var.into()).is_ok()
}

/// save the current env variabls
pub async fn save(new_vars: HashMap<&'static str, String>) -> anyhow::Result<()> {
    let target_vars = vec![
        EnvVar::ApplicationHost,
        EnvVar::ApplicationID,
        EnvVar::ApplicationName,
        EnvVar::ApplicationSecret,
        EnvVar::DatabaseUrlCore,
        EnvVar::DatabaseUrlSelf,
    ];

    let mut vars = HashMap::new();
    for var in target_vars.into_iter() {
        let v = get(var.clone());
        let k: &'static str = var.into();
        vars.insert(k, v);
    }

    for (k, v) in new_vars.iter() {
        vars.entry(k).and_modify(|m| {
            *m = v.clone();
        });
    }

    let mut lines = Vec::new();
    for (k, v) in vars.into_iter() {
        let line = format!("{k} = \"{v}\"");
        lines.push(line);
    }

    let content = lines.join("\n");
    tokio::fs::write(".env", &content).await?;

    Ok(())
}
