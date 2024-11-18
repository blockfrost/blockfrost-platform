use blockfrost_platform_macros::{define_paren_format, ParenFormat};

define_paren_format!();

#[derive(ParenFormat)]
struct NamedStruct {
    x: i32,
    y: f64,
}

#[derive(ParenFormat)]
struct TupleStruct(i32, f64);

#[derive(ParenFormat)]
struct WrapperStruct (TupleStruct);

#[derive(ParenFormat)]
struct UnitStruct;
  
#[derive(ParenFormat)]
enum ExampleEnum {
    Unit,
    TupleVariant(i32, f64),
    StructVariant { x: i32, y: f64 },
}
 
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_struct() {
        let named = NamedStruct { x: 42, y: 3.14 };
        assert_eq!(named.to_paren_string(), "NamedStruct (42 3.14)");
    }

    #[test]
    fn test_tuple_struct() {
        let tuple = TupleStruct(1, 2.71);
        assert_eq!(tuple.to_paren_string(), "TupleStruct (1 2.71)");
    }

    #[test]
    fn test_unit_struct() {
        let unit = UnitStruct;
        assert_eq!(unit.to_paren_string(), "UnitStruct");
    }
 
    #[test]
    fn test_enum_unit_variant() {
        let variant = ExampleEnum::Unit;
        assert_eq!(variant.to_paren_string(), "Unit");
    }

    #[test]
    fn test_enum_tuple_variant() {
        let variant = ExampleEnum::TupleVariant(7, 1.23);
        assert_eq!(variant.to_paren_string(), "TupleVariant (7 1.23)");
    }

    #[test]
    fn test_enum_struct_variant() {
        let variant = ExampleEnum::StructVariant { x: 5, y: 2.0 };
        assert_eq!(variant.to_paren_string(), "StructVariant (5 2)");
    }
     
    #[test]
    fn test_wrapper_struct() {
        let wrapper = WrapperStruct(TupleStruct(1, 2.71));
        assert_eq!(wrapper.to_paren_string(), "WrapperStruct (TupleStruct (1 2.71))");
    }
}
