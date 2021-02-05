use crate::error::FbapiError;
use serde::Serialize;

use std::borrow::Cow;

pub struct BatchRequest {
    inner: serde_json::Value,
    pub batch_count: usize,
}

impl Into<serde_json::Value> for BatchRequest {
    fn into(self) -> serde_json::Value {
        self.inner
    }
}

impl ToString for BatchRequest {
    fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

pub struct Builder {
    items: Vec<Item>,
}

impl Builder {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    /// Add a GET, suitable for method chaining through into build()
    #[inline]
    pub fn get<'a, StrOrString: Into<Cow<'a, str>>>(
        mut self,
        relative_url: StrOrString,
        params: &[(&str, &str)],
    ) -> Self {
        self.add_get(relative_url, params);
        self
    }

    /// Add a GET, suitable for use inside loop
    pub fn add_get<'a, StrOrString: Into<Cow<'a, str>>>(
        &mut self,
        relative_url: StrOrString,
        params: &[(&str, &str)],
    ) {
        self.add_get_internal(None, relative_url.into(), params);
    }

    /// Add a named GET, suitable for method chaining through into build()
    #[inline]
    pub fn get_with_name<
        'a,
        'b,
        StrOrString1: Into<Cow<'a, str>>,
        StrOrString2: Into<Cow<'b, str>>,
    >(
        mut self,
        name: StrOrString1,
        relative_url: StrOrString2,
        params: &[(&str, &str)],
        response_on_success: ResponseOnSuccess,
    ) -> Self {
        self.add_get_with_name(name, relative_url, params, response_on_success);
        self
    }

    /// Add a named GET, suitable for use inside loop
    pub fn add_get_with_name<
        'a,
        'b,
        StrOrString1: Into<Cow<'a, str>>,
        StrOrString2: Into<Cow<'b, str>>,
    >(
        &mut self,
        name: StrOrString1,
        relative_url: StrOrString2,
        params: &[(&str, &str)],
        response_on_success: ResponseOnSuccess,
    ) {
        self.add_get_internal(
            Some((name.into(), response_on_success)),
            relative_url.into(),
            params,
        );
    }

    fn add_get_internal<'a, 'b>(
        &mut self,
        name: Option<(Cow<'a, str>, ResponseOnSuccess)>,
        relative_url: Cow<'b, str>,
        params: &[(&str, &str)],
    ) {
        self.items
            .push(Item::Get(ItemCommon::new(name, relative_url, params)))
    }

    pub fn build(self) -> Result<BatchRequest, FbapiError> {
        let batch_count = self.items.len();
        Ok(BatchRequest {
            inner: serde_json::to_value(self.items)?,
            batch_count: batch_count,
        })
    }
}

pub enum ResponseOnSuccess {
    /// omit_response_on_success=true
    Omit,
    /// omit_response_on_success=false
    Preserve,
}

#[derive(Serialize, Debug, Clone)]
struct ItemCommon {
    relative_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    omit_response_on_success: Option<bool>,
}

impl ItemCommon {
    #[inline]
    fn new<'a, 'b>(
        name: Option<(Cow<'a, str>, ResponseOnSuccess)>,
        relative_url: Cow<'b, str>,
        params: &[(&str, &str)],
    ) -> Self {
        let (name, omit_response_on_success) = match name {
            Some((name, response_on_success)) => (
                Some(name.into_owned()),
                Some(match response_on_success {
                    ResponseOnSuccess::Omit => true,
                    ResponseOnSuccess::Preserve => false,
                }),
            ),
            None => (None, None),
        };
        Self {
            relative_url: Self::relative_url_with_params(relative_url, params),
            name,
            omit_response_on_success,
        }
    }

    fn relative_url_with_params(relative_url: Cow<'_, str>, params: &[(&str, &str)]) -> String {
        if params.is_empty() {
            relative_url.into_owned()
        } else {
            let query = params
                .into_iter()
                .map(|&(key, value)| format!("{}={}", key, value))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", &relative_url, query)
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "method", rename_all = "UPPERCASE")]
#[allow(dead_code)]
enum Item {
    Get(ItemCommon),
    Delete(ItemCommon),
    Post {
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(flatten)]
        common: ItemCommon,
    },
    Put {
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(flatten)]
        common: ItemCommon,
    },
}

pub(crate) fn response_shaper(
    res: serde_json::Value,
) -> Result<Vec<Result<serde_json::Value, FbapiError>>, FbapiError> {
    let list = match res {
        serde_json::Value::Array(vec) => vec,
        other => return Err(FbapiError::UnExpected(other)),
    };
    Ok(list
        .into_iter()
        .map(|json| {
            let body: serde_json::Value = match &json["body"] {
                serde_json::Value::String(body) => serde_json::from_str(body)?,
                _ if json.is_null() => serde_json::Value::Null, // either timeout or omit_response_on_success
                _ => return Err(FbapiError::UnExpected(json)),
            };
            if body["error"].is_object() {
                Err(FbapiError::Facebook(body))
            } else {
                Ok(body)
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_without_body() -> Result<(), FbapiError> {
        let batch = Builder::new()
            .get("foo", &[("one", "uno"), ("two", "dos")])
            .get_with_name("bar", "baz", &[], ResponseOnSuccess::Preserve)
            .build()?;
        let json: serde_json::Value = batch.into();
        assert_eq!(
            json,
            json!([
                {
                    "relative_url": "foo?one=uno&two=dos",
                    "method": "GET",
                },
                {
                    "relative_url": "baz",
                    "method": "GET",
                    "name": "bar",
                    "omit_response_on_success": false,
                }
            ])
        );

        Ok(())
    }

    #[test]
    fn test_item_with_body() -> Result<(), serde_json::Error> {
        let item = Item::Post {
            body: Some("jugemu jugemu".to_string()),
            common: ItemCommon {
                relative_url: "bar".to_string(),
                name: Some("named".to_string()),
                omit_response_on_success: None,
            },
        };
        let json = serde_json::to_value(item)?;
        assert_eq!(
            json,
            json!({
                "relative_url": "bar",
                "name": "named",
                "method": "POST",
                "body": "jugemu jugemu",
            })
        );

        Ok(())
    }

    #[test]
    fn test_response_shaper() -> Result<(), FbapiError> {
        let error_json = json!({
            "error": {
                "type": "OAuthException",
                "code": 42,
            },
        });
        let error_body = error_json.to_string();
        let ok_json = json!({
            "id": "141421356",
            "name": "sqrt(2)",
        });
        let ok_body = ok_json.to_string();
        let responses = json!([
            null,
            {
                "code": 403,
                "body": error_body,
            },
            {
                "code": 200,
                "body": ok_body,
            },
        ]);
        let results = response_shaper(responses)?;
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].as_ref().ok(), Some(&serde_json::Value::Null));
        assert_eq!(
            match &results[1] {
                Err(FbapiError::Facebook(value)) => value,
                _ => &serde_json::Value::Null,
            },
            &error_json
        );
        assert_eq!(results[2].as_ref().ok(), Some(&ok_json));
        Ok(())
    }
}
