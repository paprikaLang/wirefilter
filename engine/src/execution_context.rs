use failure::Fail;
use scheme::{Field, Scheme};
use types::{GetType, LhsValue, Type};

/// An error that occurs if the type of the value for the field doesn't
/// match the type specified in the [`Scheme`](struct@Scheme).
#[derive(Debug, PartialEq, Fail)]
#[fail(
    display = "the field should have {:?} type, but {:?} was provided",
    field_type, value_type
)]
pub struct FieldValueTypeMismatchError {
    /// The type of the field specified in the [`Scheme`](struct@Scheme).
    pub field_type: Type,
    /// Provided value type.
    pub value_type: Type,
}

/// An execution context stores an associated [`Scheme`](struct@Scheme) and a
/// set of runtime values to execute [`Filter`](::Filter) against.
///
/// It acts as a map in terms of public API, but provides a constant-time
/// index-based access to values for a filter during execution.
pub struct ExecutionContext<'e> {
    scheme: &'e Scheme,
    values: Box<[Option<LhsValue<'e>>]>,
}

impl<'e> ExecutionContext<'e> {
    /// Creates an execution context associated with a given scheme.
    ///
    /// This scheme will be used for resolving any field names and indices.
    pub fn new<'s: 'e>(scheme: &'s Scheme) -> Self {
        ExecutionContext {
            scheme,
            values: vec![None; scheme.get_field_count()].into(),
        }
    }

    /// Returns an associated scheme.
    pub fn scheme(&self) -> &'e Scheme {
        self.scheme
    }

    pub(crate) fn get_field_value_unchecked(&self, field: Field<'e>) -> &LhsValue<'e> {
        // This is safe because this code is reachable only from Filter::execute
        // which already performs the scheme compatibility check, but check that
        // invariant holds in the future at least in the debug mode.
        debug_assert!(self.scheme() == field.scheme());

        // For now we panic in this, but later we are going to align behaviour
        // with wireshark: resolve all subexpressions that don't have RHS value
        // to `false`.
        self.values[field.index()].as_ref().unwrap_or_else(|| {
            panic!(
                "Field {} was registered but not given a value",
                field.name()
            );
        })
    }

    /// Sets a runtime value for a given field name.
    pub fn set_field_value<'v: 'e, V: Into<LhsValue<'v>>>(
        &mut self,
        name: &str,
        value: V,
    ) -> Result<(), FieldValueTypeMismatchError> {
        let field = self.scheme.get_field_index(name).unwrap();
        let value = value.into();

        let field_type = field.get_type();
        let value_type = value.get_type();

        if field_type == value_type {
            self.values[field.index()] = Some(value);
            Ok(())
        } else {
            Err(FieldValueTypeMismatchError {
                field_type,
                value_type,
            })
        }
    }
}

#[test]
fn test_field_value_type_mismatch() {
    let scheme = Scheme! { foo: Int };

    let mut ctx = ExecutionContext::new(&scheme);

    assert_eq!(
        ctx.set_field_value("foo", LhsValue::Bool(false)),
        Err(FieldValueTypeMismatchError {
            field_type: Type::Int,
            value_type: Type::Bool
        })
    );
}
