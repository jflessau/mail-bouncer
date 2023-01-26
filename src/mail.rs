use crate::error::{Error, Result};
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use std::env;

pub async fn send_mail(text: String) -> Result<()> {
    let smtp_relay = env::var("SMTP_RELAY").unwrap();
    let smtp_user = env::var("SMTP_USER").unwrap();
    let smtp_password = env::var("SMTP_PASSWORD").unwrap();
    let from = env::var("FROM").unwrap();
    let to = env::var("TO").unwrap();
    let subject = env::var("SUBJECT").unwrap();

    let email = Message::builder()
        .from(from.parse().unwrap())
        .to(to.parse().unwrap())
        .subject(subject)
        .body(text)
        .unwrap();

    let creds = Credentials::new(smtp_user, smtp_password);

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_relay)
            .unwrap()
            .credentials(creds)
            .build();

    if let Err(err) = mailer.send(email).await {
        return Err(Error::InternalServer(format!("{err}")));
    }

    Ok(())
}
