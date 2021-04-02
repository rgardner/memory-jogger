//! Provides the Email API.
//!
//! Uses [SendGrid](https://sendgrid.com) for sending emails.

use std::fmt;

use anyhow::Result;
use serde::Serialize;

use crate::http;

pub struct SendGridApiClient<'a> {
    sendgrid_api_key: String,
    client: &'a reqwest::Client,
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

impl<'a> SendGridApiClient<'a> {
    pub fn new(sendgrid_api_key: String, client: &'a reqwest::Client) -> Self {
        Self {
            sendgrid_api_key,
            client,
        }
    }

    /// Sends email.
    pub async fn send(&self, mail: Mail) -> Result<()> {
        // https://sendgrid.com/docs/API_Reference/Web_API_v3/Mail/index.html
        let url = build_mail_send_url();
        let body: SendMailRequestBody = mail.into();
        self.client
            .post(url)
            .bearer_auth(&self.sendgrid_api_key)
            .header(reqwest::header::CONTENT_TYPE, http::CONTENT_TYPE_JSON)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

fn build_mail_send_url() -> reqwest::Url {
    reqwest::Url::parse("https://api.sendgrid.com/v3/mail/send").unwrap()
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

/// Email identity.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mail_send_url_returns_nonempty_string() {
        let url = build_mail_send_url();
        assert!(!url.as_str().is_empty());
    }
}
