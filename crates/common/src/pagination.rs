use crate::errors::BlockfrostError;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

impl Order {
    pub fn from_string(order: Option<String>) -> Result<Order, &'static str> {
        let order = order.unwrap_or_else(|| "asc".to_string());

        if order != "asc" && order != "desc" {
            return Err("querystring/order must be equal to one of the allowed values");
        }

        match order.as_str() {
            "asc" => Ok(Order::Asc),
            "desc" => Ok(Order::Desc),
            _ => Ok(Order::Asc),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Order::Asc => "asc",
            Order::Desc => "desc",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<String>,
    pub count: Option<String>,
    pub order: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub page: i32,
    pub count: i32,
    pub order: Order,
    pub from: ParamParts,
    pub to: ParamParts,
}

impl Pagination {
    pub async fn from_query(query: PaginationQuery) -> Result<Self, BlockfrostError> {
        let count = get_count_param(query.count.clone());
        let count = match count {
            Ok(count) => count,
            Err(e) => return Err(BlockfrostError::custom_400(e.to_string())),
        };

        let page = get_page_param(query.page.clone());
        let page = match page {
            Ok(page) => page,
            Err(e) => return Err(BlockfrostError::custom_400(e.to_string())),
        };
        let order = get_order_param(query.order.clone());
        let order = match order {
            Ok(order) => order,
            Err(e) => return Err(BlockfrostError::custom_400(e.to_string())),
        };

        // from -> to parameters
        let from_result = get_range_param(query.from.clone());
        let from = match from_result {
            Ok(from) => from,
            Err(_) => return Err(BlockfrostError::malformed_range_param()),
        };

        let to_result = get_range_param(query.to.clone());
        let to = match to_result {
            Ok(to) => to,
            Err(_) => return Err(BlockfrostError::malformed_range_param()),
        };

        if to.height.is_some() && from.height.is_some() && from.height > to.height {
            return Err(BlockfrostError::malformed_range_param());
        }

        Ok(Pagination {
            count,
            page,
            order,
            from,
            to,
        })
    }
}

pub fn get_order_param(param: Option<String>) -> Result<Order, &'static str> {
    let order = param.unwrap_or_else(|| "asc".to_string());

    if order != "asc" && order != "desc" {
        return Err("querystring/order must be equal to one of the allowed values");
    }

    Order::from_string(Some(order))
}

pub fn get_page_param(param: Option<String>) -> Result<i32, &'static str> {
    let page = param.unwrap_or_else(|| "1".to_string());

    if !page.chars().all(|c| c.is_ascii_digit()) {
        return Err("querystring/page must be integer");
    }

    let page: i32 = page
        .parse()
        .map_err(|_| "querystring/page must be <= 21474836")?;

    if page < 1 {
        return Err("querystring/page must be >= 1");
    }

    if page > 21474836 {
        return Err("querystring/page must be <= 21474836");
    }

    Ok(page)
}

pub fn get_count_param(param: Option<String>) -> Result<i32, &'static str> {
    let count = param.unwrap_or_else(|| "100".to_string());

    // weird flex just to match the old api
    let count: i64 = count
        .parse()
        .map_err(|_| "querystring/count must be integer")?;

    if count < 1 {
        return Err("querystring/count must be >= 1");
    }

    if count > 100 {
        return Err("querystring/count must be <= 100");
    }

    let count: i32 = count
        .try_into()
        .map_err(|_| "querystring/count must be integer")?;

    Ok(count)
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ParamParts {
    pub height: Option<i32>,
    pub index: Option<i32>,
}

pub fn get_range_param(param: Option<String>) -> Result<ParamParts, BlockfrostError> {
    match param {
        Some(param_str) => {
            let parts: Vec<&str> = param_str.split(':').collect();
            let parsed_height = parts
                .first()
                .ok_or(BlockfrostError::malformed_range_param())?
                .parse::<i32>()
                .map_err(|_| BlockfrostError::malformed_range_param())?;

            if !valid_value(parsed_height) {
                return Err(BlockfrostError::malformed_range_param());
            }

            match parts.len() {
                1 => Ok(ParamParts {
                    height: Some(parsed_height),
                    index: None,
                }),
                2 => {
                    let parsed_index = parts[1]
                        .parse::<i32>()
                        .map_err(|_| BlockfrostError::malformed_range_param())?;
                    if !valid_value(parsed_index) {
                        return Err(BlockfrostError::malformed_range_param());
                    }

                    Ok(ParamParts {
                        height: Some(parsed_height),
                        index: Some(parsed_index),
                    })
                },
                _ => Err(BlockfrostError::malformed_range_param()),
            }
        },
        None => Ok(ParamParts {
            height: None,
            index: None,
        }),
    }
}

pub trait ApplyPagination {
    fn apply_pagination(&mut self, pagination: &Pagination);
}

impl ApplyPagination for Url {
    fn apply_pagination(&mut self, pagination: &Pagination) {
        let mut query_pairs = self.query_pairs_mut();

        query_pairs
            .append_pair("page", &pagination.page.to_string())
            .append_pair("count", &pagination.count.to_string())
            .append_pair("order", pagination.order.as_str());

        if let Some(from_height) = pagination.from.height {
            let from_value = if let Some(from_index) = pagination.from.index {
                format!("{from_height}:{from_index}")
            } else {
                from_height.to_string()
            };

            query_pairs.append_pair("from", &from_value);
        }

        if let Some(to_height) = pagination.to.height {
            let to_value = if let Some(to_index) = pagination.to.index {
                format!("{to_height}:{to_index}",)
            } else {
                to_height.to_string()
            };

            query_pairs.append_pair("to", &to_value);
        }
    }
}

fn valid_value(val: i32) -> bool {
    (0..=i32::MAX).contains(&val)
}

#[cfg(test)]
mod tests {
    use crate::errors::BlockfrostError;
    use crate::pagination::{
        Order, ParamParts, get_count_param, get_order_param, get_page_param, get_range_param,
    };
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(None, Ok(Order::Asc))]
    #[case(Some("desc".to_string()), Ok(Order::Desc))]
    #[case(Some("something-wrong".to_string()), Err("querystring/order must be equal to one of the allowed values"))]
    fn test_get_order_param(#[case] input: Option<String>, #[case] expected: Result<Order, &str>) {
        assert_eq!(get_order_param(input), expected);
    }

    #[rstest]
    #[case(None, Ok(1))]
    #[case(Some("1".to_string()), Ok(1))]
    #[case(Some("string".to_string()), Err("querystring/page must be integer"))]
    #[case(Some("0".to_string()), Err("querystring/page must be >= 1"))]
    #[case(Some("21474837".to_string()), Err("querystring/page must be <= 21474836"))]
    fn test_get_page_param(#[case] input: Option<String>, #[case] expected: Result<i32, &str>) {
        assert_eq!(get_page_param(input), expected);
    }

    #[rstest]
    #[case(None, Ok(100))]
    #[case(Some("101".to_string()), Err("querystring/count must be <= 100"))]
    #[case(Some("0".to_string()), Err("querystring/count must be >= 1"))]
    #[case(Some("string".to_string()), Err("querystring/count must be integer"))]
    fn test_get_count_param(#[case] input: Option<String>, #[case] expected: Result<i32, &str>) {
        assert_eq!(get_count_param(input), expected);
    }

    #[rstest]
    #[case(None, Ok(ParamParts { height: None, index: None }))]
    #[case(Some("123".to_string()), Ok(ParamParts { height: Some(123), index: None }))]
    #[case(Some("123:321".to_string()), Ok(ParamParts { height: Some(123), index: Some(321) }))]
    #[case(Some("123::321".to_string()), Err(BlockfrostError::malformed_range_param()))]
    #[case(Some("9999999999999".to_string()), Err(BlockfrostError::malformed_range_param()))]
    #[case(Some("1:9999999999999".to_string()), Err(BlockfrostError::malformed_range_param()))]
    #[case(Some("1:s".to_string()), Err(BlockfrostError::malformed_range_param()))]
    #[case(Some("-1".to_string()), Err(BlockfrostError::malformed_range_param()))]
    #[case(Some("s:1".to_string()), Err(BlockfrostError::malformed_range_param()))]
    #[case(Some("s:s".to_string()), Err(BlockfrostError::malformed_range_param()))]
    #[case(Some("-5000".to_string()), Err(BlockfrostError::malformed_range_param()))]
    #[case(Some("some-string".to_string()), Err(BlockfrostError::malformed_range_param()))]
    fn test_get_range_param(
        #[case] input: Option<String>,
        #[case] expected: Result<super::ParamParts, BlockfrostError>,
    ) {
        assert_eq!(get_range_param(input), expected);
    }

    #[rstest]
    #[case(
        "https://blockfrost.io",
        2,
        50,
        Order::Desc,
        None,
        None,
        "https://blockfrost.io/?page=2&count=50&order=desc"
    )]
    #[case(
            "https://blockfrost.io/endpoint",
            1,
            100,
            Order::Asc,
            Some((123, None)),
            None,
            "https://blockfrost.io/endpoint?page=1&count=100&order=asc&from=123"
        )]
    #[case(
            "https://blockfrost.io/endpoint",
            1,
            100,
            Order::Asc,
            Some((123, Some(7))),
            None,
            "https://blockfrost.io/endpoint?page=1&count=100&order=asc&from=123%3A7"
        )]
    #[case(
            "https://blockfrost.io/endpoint",
            1,
            100,
            Order::Asc,
            None,
            Some((200, Some(5))),
            "https://blockfrost.io/endpoint?page=1&count=100&order=asc&to=200%3A5"
        )]
    #[case(
            "https://blockfrost.io/endpoint",
            3,
            20,
            Order::Desc,
            Some((10, Some(2))),
            Some((50, Some(8))),
            "https://blockfrost.io/endpoint?page=3&count=20&order=desc&from=10%3A2&to=50%3A8"
        )]
    #[case(
            "https://example.com/transactions",
            1,
            6,
            Order::Asc,
            Some((723665, Some(24))),
            Some((723665, Some(29))),
            "https://example.com/transactions?page=1&count=6&order=asc&from=723665%3A24&to=723665%3A29"
        )]
    fn test_apply_pagination(
        #[case] base: &str,
        #[case] page: i32,
        #[case] count: i32,
        #[case] order: Order,
        #[case] from: Option<(i32, Option<i32>)>,
        #[case] to: Option<(i32, Option<i32>)>,
        #[case] expected: &str,
    ) {
        use crate::pagination::{ApplyPagination, Pagination};
        use url::Url;

        let mut url = Url::parse(base).unwrap();

        let from_parts = from
            .map(|(h, idx)| ParamParts {
                height: Some(h),
                index: idx,
            })
            .unwrap_or(ParamParts {
                height: None,
                index: None,
            });

        let to_parts = to
            .map(|(h, idx)| ParamParts {
                height: Some(h),
                index: idx,
            })
            .unwrap_or(ParamParts {
                height: None,
                index: None,
            });

        let pagination = Pagination {
            page,
            count,
            order,
            from: from_parts,
            to: to_parts,
        };

        url.apply_pagination(&pagination);

        assert_eq!(url.as_str(), expected);
    }
}
