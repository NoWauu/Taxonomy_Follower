use crate::mail::Email;

pub fn password_reset(recipient: &str, reset_link: &str, valid_for_minutes: i64) -> Email {
    let text_body = format!(
        "Hi,\n\n\
         We received a request to reset the password of your Taxonomy Follower account.\n\n\
         Open this link to choose a new password:\n{reset_link}\n\n\
         The link expires in {valid_for_minutes} minutes and can only be used once.\n\n\
         If you did not ask for this, you can ignore this email; your password stays unchanged.\n"
    );

    let html_body = format!(
        r#"<!doctype html>
<html>
  <body style="margin:0;padding:24px;background:#f5f5f4;font-family:system-ui,-apple-system,Segoe UI,sans-serif;color:#1c1917;">
    <div style="max-width:520px;margin:0 auto;background:#ffffff;border-radius:12px;padding:32px;">
      <h1 style="margin:0 0 16px;font-size:20px;">Reset your password</h1>
      <p style="margin:0 0 16px;line-height:1.6;">
        We received a request to reset the password of your Taxonomy Follower account.
      </p>
      <p style="margin:0 0 24px;">
        <a href="{reset_link}"
           style="display:inline-block;padding:12px 20px;border-radius:8px;background:#166534;color:#ffffff;text-decoration:none;font-weight:600;">
          Choose a new password
        </a>
      </p>
      <p style="margin:0 0 16px;line-height:1.6;color:#57534e;font-size:14px;">
        The link expires in {valid_for_minutes} minutes and can only be used once.
        If you did not ask for this, you can ignore this email &mdash; your password stays unchanged.
      </p>
      <p style="margin:0;line-height:1.6;color:#78716c;font-size:12px;word-break:break-all;">
        If the button does not work, paste this into your browser:<br>{reset_link}
      </p>
    </div>
  </body>
</html>"#
    );

    Email {
        to: recipient.to_string(),
        subject: "Reset your Taxonomy Follower password".to_string(),
        text_body,
        html_body,
    }
}
