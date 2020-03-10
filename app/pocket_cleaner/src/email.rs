//! A module for sending emails.

use std::fmt;

use actix_web::{
    client::{Client, ClientBuilder},
    http::{header::ContentType, uri::Uri, PathAndQuery},
};
use serde::Serialize;

use crate::error::{PocketCleanerError, Result};

pub struct SendGridAPIClient {
    sendgrid_api_key: String,
}

#[derive(Clone, Debug)]
pub struct Mail {
    pub from_email: String,
    pub to_email: String,
    pub subject: String,
    pub html_content: String,
}

impl fmt::Display for Mail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "From: {}\nTo: {}\nSubject: {}\n\n{}",
            self.from_email, self.to_email, self.subject, self.html_content
        )
    }
}

impl SendGridAPIClient {
    pub fn new(sendgrid_api_key: String) -> Self {
        Self { sendgrid_api_key }
    }

    pub async fn send(&self, mail: &Mail) -> Result<()> {
        let client = ClientBuilder::new()
            .bearer_auth(&self.sendgrid_api_key)
            .finish();
        let req = SendMailRequest { mail: mail.clone() };
        send_send_mail_request(&client, &req).await?;
        Ok(())
    }
}

struct SendMailRequest {
    mail: Mail,
}

#[derive(Serialize)]
struct SendMailRequestBody {
    personalizations: Vec<MailPersonalization>,
    from: Email,
    subject: String,
    content: Vec<ContentTypeAndValue>,
}

#[derive(Serialize)]
struct MailPersonalization {
    to: Vec<Email>,
}

#[derive(Serialize)]
struct Email {
    email: String,
}

#[derive(Serialize)]
struct ContentTypeAndValue {
    r#type: String,
    value: String,
}

impl From<Mail> for SendMailRequestBody {
    fn from(mail: Mail) -> Self {
        Self {
            personalizations: vec![MailPersonalization {
                to: vec![Email::new(mail.to_email)],
            }],
            from: Email::new(mail.from_email),
            subject: mail.subject,
            content: vec![ContentTypeAndValue::new(
                ContentType::html(),
                mail.html_content,
            )],
        }
    }
}

impl Email {
    fn new(email: String) -> Self {
        Self { email }
    }
}

impl ContentTypeAndValue {
    fn new(content_type: ContentType, value: String) -> Self {
        Self {
            r#type: content_type.to_string(),
            value,
        }
    }
}

fn build_mail_send_url() -> Result<Uri> {
    let path_and_query: PathAndQuery = "/v3/mail/send".parse().unwrap();
    Ok(Uri::builder()
        .scheme("https")
        .authority("api.sendgrid.com")
        .path_and_query(path_and_query)
        .build()
        .map_err(|e| PocketCleanerError::Logic(e.to_string()))?)
}

async fn send_send_mail_request(client: &Client, req: &SendMailRequest) -> Result<()> {
    let url = build_mail_send_url()?;
    let body: SendMailRequestBody = req.mail.clone().into();
    let mut resp = client
        .post(url)
        .content_type("application/json")
        .send_json(&body)
        .await
        .map_err(|e| PocketCleanerError::Unknown(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .body()
            .await
            .map_err(|e| PocketCleanerError::Unknown(e.to_string()))?;
        let body =
            std::str::from_utf8(&body).map_err(|e| PocketCleanerError::Unknown(e.to_string()))?;
        log::error!(
            "SendGrid Send Mail HTTP request failed (HTTP {}): {}",
            status,
            body
        );
        return Err(PocketCleanerError::Unknown(
            "SendGrid send mail HTTP request failed".into(),
        ));
    }

    Ok(())
}
