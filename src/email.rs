//! A module for sending emails.

use std::fmt;

use serde::Serialize;

use crate::{
    error::{Error, Result},
    http,
};

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
        let req = SendMailRequest { mail: mail.clone() };
        send_send_mail_request(&self.sendgrid_api_key, &req).await?;
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
                http::CONTENT_TYPE_HTML.into(),
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
    fn new(content_type: String, value: String) -> Self {
        Self {
            r#type: content_type,
            value,
        }
    }
}

fn build_mail_send_url() -> Result<reqwest::Url> {
    let url = reqwest::Url::parse("https://api.sendgrid.com/v3/mail/send").unwrap();
    Ok(url)
}

async fn send_send_mail_request(api_key: &str, req: &SendMailRequest) -> Result<()> {
    let url = build_mail_send_url()?;
    let body: SendMailRequestBody = req.mail.clone().into();
    let resp = reqwest::Client::new()
        .post(url)
        .bearer_auth(api_key)
        .header(reqwest::header::CONTENT_TYPE, http::CONTENT_TYPE_JSON)
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Unknown(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Unknown(e.to_string()))?;
        log::error!(
            "SendGrid Send Mail HTTP request failed (HTTP {}): {}",
            status,
            body
        );
        return Err(Error::Unknown(
            "SendGrid send mail HTTP request failed".into(),
        ));
    }

    Ok(())
}
