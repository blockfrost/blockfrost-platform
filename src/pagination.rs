use crate::errors::BlockfrostError;
use serde::{Deserialize, Serialize};

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
}
