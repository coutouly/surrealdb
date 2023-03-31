pub(super) mod map;
pub(super) mod opt;
pub(super) mod vec;

use crate::err::Error;
use crate::sql;
use crate::sql::array::Array;
use crate::sql::object::Object;
use crate::sql::serde::serialize_internal;
use crate::sql::value::serde::ser;
use crate::sql::value::Value;
use crate::sql::Block;
use crate::sql::Datetime;
use crate::sql::Duration;
use crate::sql::Future;
use crate::sql::Ident;
use crate::sql::Idiom;
use crate::sql::Param;
use crate::sql::Regex;
use crate::sql::Strand;
use crate::sql::Table;
use crate::sql::Uuid;
use bigdecimal::BigDecimal;
use bigdecimal::FromPrimitive;
use map::SerializeValueMap;
use ser::edges::SerializeEdges;
use ser::expression::SerializeExpression;
use ser::function::SerializeFunction;
use ser::model::SerializeModel;
use ser::range::SerializeRange;
use ser::thing::SerializeThing;
use ser::Serializer as _;
use serde::ser::Error as _;
use serde::ser::Serialize;
use serde::ser::SerializeMap as _;
use serde::ser::SerializeSeq as _;
use std::fmt::Display;
use storekey::encode::Error as EncodeError;
use vec::SerializeValueVec;

/// Convert a `T` into `surrealdb::sql::Value` which is an enum that can represent any valid SQL data.
pub(crate) fn to_value<T>(value: T) -> Result<Value, Error>
where
	T: Serialize,
{
	serialize_internal(|| value.serialize(Serializer.wrap()))
}

impl serde::ser::Error for Error {
	fn custom<T>(msg: T) -> Self
	where
		T: Display,
	{
		Self::Encode(EncodeError::Message(msg.to_string()))
	}
}

pub(super) struct Serializer;

impl ser::Serializer for Serializer {
	type Ok = Value;
	type Error = Error;

	type SerializeSeq = SerializeArray;
	type SerializeTuple = SerializeArray;
	type SerializeTupleStruct = SerializeArray;
	type SerializeTupleVariant = SerializeTupleVariant;
	type SerializeMap = SerializeMap;
	type SerializeStruct = SerializeStruct;
	type SerializeStructVariant = SerializeStructVariant;

	const EXPECTED: &'static str = "an enum `Value`";

	#[inline]
	fn serialize_bool(self, value: bool) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_i8(self, value: i8) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_i16(self, value: i16) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_i32(self, value: i32) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_i64(self, value: i64) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	fn serialize_i128(self, value: i128) -> Result<Self::Ok, Error> {
		match BigDecimal::from_i128(value) {
			Some(decimal) => Ok(decimal.into()),
			None => Err(Error::TryFromError(value.to_string(), "BigDecimal")),
		}
	}

	#[inline]
	fn serialize_u8(self, value: u8) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_u16(self, value: u16) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_u32(self, value: u32) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_u64(self, value: u64) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	fn serialize_u128(self, value: u128) -> Result<Self::Ok, Error> {
		match BigDecimal::from_u128(value) {
			Some(decimal) => Ok(decimal.into()),
			None => Err(Error::TryFromError(value.to_string(), "BigDecimal")),
		}
	}

	#[inline]
	fn serialize_f32(self, value: f32) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_f64(self, value: f64) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	#[inline]
	fn serialize_char(self, value: char) -> Result<Self::Ok, Error> {
		Ok({
			let mut s = String::new();
			s.push(value);
			s
		}
		.into())
	}

	#[inline]
	fn serialize_str(self, value: &str) -> Result<Self::Ok, Error> {
		Ok(value.into())
	}

	fn serialize_bytes(self, values: &[u8]) -> Result<Self::Ok, Error> {
		let mut vec = Vec::with_capacity(values.len());
		for value in values {
			vec.push(Value::from(*value));
		}
		Ok(vec.into())
	}

	#[inline]
	fn serialize_unit(self) -> Result<Self::Ok, Error> {
		Ok(Value::None)
	}

	#[inline]
	fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Error> {
		self.serialize_unit()
	}

	#[inline]
	fn serialize_unit_variant(
		self,
		name: &'static str,
		variant_index: u32,
		variant: &'static str,
	) -> Result<Self::Ok, Error> {
		match name {
			sql::constant::TOKEN => ser::constant::Serializer
				.serialize_unit_variant(name, variant_index, variant)
				.map(Value::Constant),
			sql::value::TOKEN => match variant {
				"None" => Ok(Value::None),
				"Null" => Ok(Value::Null),
				"False" => Ok(Value::False),
				"True" => Ok(Value::True),
				variant => Err(Error::custom(format!("unknown unit variant `Value::{variant}`"))),
			},
			_ => self.serialize_str(variant),
		}
	}

	#[inline]
	fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Self::Ok, Error>
	where
		T: ?Sized + Serialize,
	{
		match name {
			sql::strand::TOKEN => {
				Ok(Value::Strand(Strand(value.serialize(ser::string::Serializer.wrap())?)))
			}
			sql::block::TOKEN => Ok(Value::Block(Box::new(Block(
				value.serialize(ser::block::entry::vec::Serializer.wrap())?,
			)))),
			sql::duration::TOKEN => {
				Ok(Value::Duration(Duration(value.serialize(ser::duration::Serializer.wrap())?)))
			}
			sql::future::TOKEN => Ok(Value::Future(Box::new(Future(Block(
				value.serialize(ser::block::entry::vec::Serializer.wrap())?,
			))))),
			sql::regex::TOKEN => {
				Ok(Value::Regex(Regex(value.serialize(ser::string::Serializer.wrap())?)))
			}
			sql::table::TOKEN => {
				Ok(Value::Table(Table(value.serialize(ser::string::Serializer.wrap())?)))
			}
			sql::idiom::TOKEN => {
				Ok(Value::Idiom(Idiom(value.serialize(ser::part::vec::Serializer.wrap())?)))
			}
			sql::param::TOKEN => {
				Ok(Value::Param(Param(Ident(value.serialize(ser::string::Serializer.wrap())?))))
			}
			sql::array::TOKEN => Ok(Value::Array(Array(value.serialize(vec::Serializer.wrap())?))),
			sql::object::TOKEN => {
				Ok(Value::Object(Object(value.serialize(map::Serializer.wrap())?)))
			}
			sql::uuid::TOKEN => {
				Ok(Value::Uuid(Uuid(value.serialize(ser::uuid::Serializer.wrap())?)))
			}
			sql::datetime::TOKEN => {
				Ok(Value::Datetime(Datetime(value.serialize(ser::datetime::Serializer.wrap())?)))
			}
			_ => value.serialize(self.wrap()),
		}
	}

	fn serialize_newtype_variant<T>(
		self,
		name: &'static str,
		variant_index: u32,
		variant: &'static str,
		value: &T,
	) -> Result<Self::Ok, Error>
	where
		T: ?Sized + Serialize,
	{
		match name {
			sql::number::TOKEN => {
				Ok(Value::Number(ser::number::Serializer.serialize_newtype_variant(
					name,
					variant_index,
					variant,
					value,
				)?))
			}
			sql::subquery::TOKEN => {
				Ok(Value::Subquery(Box::new(ser::subquery::Serializer.serialize_newtype_variant(
					name,
					variant_index,
					variant,
					value,
				)?)))
			}
			sql::geometry::TOKEN => {
				Ok(Value::Geometry(ser::geometry::Serializer.serialize_newtype_variant(
					name,
					variant_index,
					variant,
					value,
				)?))
			}
			sql::value::TOKEN => value.serialize(Serializer.wrap()),
			_ => Ok(map! {
				String::from(variant) => value.serialize(Serializer.wrap())?,
			}
			.into()),
		}
	}

	#[inline]
	fn serialize_none(self) -> Result<Self::Ok, Error> {
		self.serialize_unit()
	}

	#[inline]
	fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Error>
	where
		T: ?Sized + Serialize,
	{
		value.serialize(self.wrap())
	}

	fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
		let inner = vec::SerializeValueVec(Vec::with_capacity(len.unwrap_or_default()));
		Ok(SerializeArray(inner))
	}

	fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Error> {
		self.serialize_seq(Some(len))
	}

	fn serialize_tuple_struct(
		self,
		_name: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleStruct, Error> {
		self.serialize_seq(Some(len))
	}

	fn serialize_tuple_variant(
		self,
		name: &'static str,
		variant_index: u32,
		variant: &'static str,
		len: usize,
	) -> Result<Self::SerializeTupleVariant, Error> {
		Ok(match name {
			sql::model::TOKEN => {
				SerializeTupleVariant::Model(ser::model::Serializer.serialize_tuple_variant(
					name,
					variant_index,
					variant,
					len,
				)?)
			}
			sql::function::TOKEN => {
				SerializeTupleVariant::Function(ser::function::Serializer.serialize_tuple_variant(
					name,
					variant_index,
					variant,
					len,
				)?)
			}
			_ => SerializeTupleVariant::Unknown {
				variant,
				fields: SerializeValueVec(Vec::with_capacity(len)),
			},
		})
	}

	fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
		Ok(SerializeMap(Default::default()))
	}

	fn serialize_struct(
		self,
		name: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStruct, Error> {
		Ok(match name {
			sql::thing::TOKEN => SerializeStruct::Thing(Default::default()),
			sql::expression::TOKEN => SerializeStruct::Expression(Default::default()),
			sql::edges::TOKEN => SerializeStruct::Edges(Default::default()),
			sql::range::TOKEN => SerializeStruct::Range(Default::default()),
			_ => SerializeStruct::Unknown(Default::default()),
		})
	}

	fn serialize_struct_variant(
		self,
		_name: &'static str,
		_variant_index: u32,
		variant: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStructVariant, Error> {
		Ok(SerializeStructVariant {
			name: String::from(variant),
			map: Object::default(),
		})
	}
}

pub(super) struct SerializeArray(vec::SerializeValueVec);

impl serde::ser::SerializeSeq for SerializeArray {
	type Ok = Value;
	type Error = Error;

	fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
	where
		T: ?Sized + Serialize,
	{
		self.0.serialize_element(value)
	}

	fn end(self) -> Result<Value, Error> {
		Ok(Value::Array(Array(self.0.end()?)))
	}
}

impl serde::ser::SerializeTuple for SerializeArray {
	type Ok = Value;
	type Error = Error;

	fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
	where
		T: ?Sized + Serialize,
	{
		serde::ser::SerializeSeq::serialize_element(self, value)
	}

	fn end(self) -> Result<Value, Error> {
		serde::ser::SerializeSeq::end(self)
	}
}

impl serde::ser::SerializeTupleStruct for SerializeArray {
	type Ok = Value;
	type Error = Error;

	fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
	where
		T: ?Sized + Serialize,
	{
		serde::ser::SerializeSeq::serialize_element(self, value)
	}

	fn end(self) -> Result<Value, Error> {
		serde::ser::SerializeSeq::end(self)
	}
}

pub(super) struct SerializeMap(map::SerializeValueMap);

impl serde::ser::SerializeMap for SerializeMap {
	type Ok = Value;
	type Error = Error;

	fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
	where
		T: ?Sized + Serialize,
	{
		self.0.serialize_key(key)
	}

	fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
	where
		T: ?Sized + Serialize,
	{
		self.0.serialize_value(value)
	}

	fn end(self) -> Result<Value, Error> {
		Ok(Value::Object(Object(self.0.end()?)))
	}
}

pub(super) enum SerializeTupleVariant {
	Model(SerializeModel),
	Function(SerializeFunction),
	Unknown {
		variant: &'static str,
		fields: SerializeValueVec,
	},
}

impl serde::ser::SerializeTupleVariant for SerializeTupleVariant {
	type Ok = Value;
	type Error = Error;

	fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
	where
		T: ?Sized + Serialize,
	{
		match self {
			SerializeTupleVariant::Model(model) => model.serialize_field(value),
			SerializeTupleVariant::Function(function) => function.serialize_field(value),
			SerializeTupleVariant::Unknown {
				ref mut fields,
				..
			} => fields.serialize_element(value),
		}
	}

	fn end(self) -> Result<Value, Error> {
		match self {
			SerializeTupleVariant::Model(model) => Ok(Value::Model(model.end()?)),
			SerializeTupleVariant::Function(function) => {
				Ok(Value::Function(Box::new(function.end()?)))
			}
			SerializeTupleVariant::Unknown {
				variant,
				fields,
			} => Ok(map! {
				variant.to_owned() => Value::Array(Array(fields.end()?)),
			}
			.into()),
		}
	}
}

pub(super) enum SerializeStruct {
	Thing(SerializeThing),
	Expression(SerializeExpression),
	Edges(SerializeEdges),
	Range(SerializeRange),
	Unknown(SerializeValueMap),
}

impl serde::ser::SerializeStruct for SerializeStruct {
	type Ok = Value;
	type Error = Error;

	fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
	where
		T: ?Sized + Serialize,
	{
		match self {
			SerializeStruct::Thing(thing) => thing.serialize_field(key, value),
			SerializeStruct::Expression(expr) => expr.serialize_field(key, value),
			SerializeStruct::Edges(edges) => edges.serialize_field(key, value),
			SerializeStruct::Range(range) => range.serialize_field(key, value),
			SerializeStruct::Unknown(map) => map.serialize_entry(key, value),
		}
	}

	fn end(self) -> Result<Value, Error> {
		match self {
			SerializeStruct::Thing(thing) => Ok(Value::Thing(thing.end()?)),
			SerializeStruct::Expression(expr) => Ok(Value::Expression(Box::new(expr.end()?))),
			SerializeStruct::Edges(edges) => Ok(Value::Edges(Box::new(edges.end()?))),
			SerializeStruct::Range(range) => Ok(Value::Range(Box::new(range.end()?))),
			SerializeStruct::Unknown(map) => Ok(Value::Object(Object(map.end()?))),
		}
	}
}

pub(super) struct SerializeStructVariant {
	name: String,
	map: Object,
}

impl serde::ser::SerializeStructVariant for SerializeStructVariant {
	type Ok = Value;
	type Error = Error;

	fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
	where
		T: ?Sized + Serialize,
	{
		self.map.0.insert(String::from(key), value.serialize(Serializer.wrap())?);
		Ok(())
	}

	fn end(self) -> Result<Value, Error> {
		let mut object = Object::default();

		object.insert(self.name, Value::Object(self.map));

		Ok(Value::Object(object))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sql;
	use crate::sql::block::Entry;
	use crate::sql::constant::Constant;
	use crate::sql::statements::CreateStatement;
	use crate::sql::*;
	use ::serde::Serialize;
	use std::ops::Bound;

	#[test]
	fn none() {
		let expected = Value::None;
		assert_eq!(expected, to_value(&None::<u32>).unwrap());
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn null() {
		let expected = Value::Null;
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn r#false() {
		let expected = Value::False;
		assert_eq!(expected, to_value(&false).unwrap());
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn r#true() {
		let expected = Value::True;
		assert_eq!(expected, to_value(&true).unwrap());
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn number() {
		let number = Number::Int(Default::default());
		let value = to_value(&number).unwrap();
		let expected = Value::Number(number);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());

		let number = Number::Float(Default::default());
		let value = to_value(&number).unwrap();
		let expected = Value::Number(number);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());

		let number = Number::Decimal(Default::default());
		let value = to_value(&number).unwrap();
		let expected = Value::Number(number);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn strand() {
		let strand = Strand("foobar".to_owned());
		let value = to_value(&strand).unwrap();
		let expected = Value::Strand(strand);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());

		let strand = "foobar".to_owned();
		let value = to_value(&strand).unwrap();
		let expected = Value::Strand(Strand(strand));
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());

		let strand = "foobar";
		let value = to_value(&strand).unwrap();
		let expected = Value::Strand(Strand(strand.to_owned()));
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn duration() {
		let duration = Duration::default();
		let value = to_value(&duration).unwrap();
		let expected = Value::Duration(duration);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn datetime() {
		let datetime = Datetime::default();
		let value = to_value(&datetime).unwrap();
		let expected = Value::Datetime(datetime);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn uuid() {
		let uuid = Uuid::default();
		let value = to_value(&uuid).unwrap();
		let expected = Value::Uuid(uuid);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn array() {
		let array = Array::default();
		let value = to_value(&array).unwrap();
		let expected = Value::Array(array);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn object() {
		let object = Object::default();
		let value = to_value(&object).unwrap();
		let expected = Value::Object(object);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn geometry() {
		let geometry = Geometry::Collection(Vec::new());
		let value = to_value(&geometry).unwrap();
		let expected = Value::Geometry(geometry);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn param() {
		let param = Param::default();
		let value = to_value(&param).unwrap();
		let expected = Value::Param(param);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn idiom() {
		let idiom = Idiom::default();
		let value = to_value(&idiom).unwrap();
		let expected = Value::Idiom(idiom);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn table() {
		let table = Table("foo".to_owned());
		let value = to_value(&table).unwrap();
		let expected = Value::Table(table);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn thing() {
		let record_id = sql::thing("foo:bar").unwrap();
		let value = to_value(&record_id).unwrap();
		let expected = Value::Thing(record_id);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn model() {
		let model = Model::Count("foo".to_owned(), Default::default());
		let value = to_value(&model).unwrap();
		let expected = Value::Model(model);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn regex() {
		let regex = Regex::default();
		let value = to_value(&regex).unwrap();
		let expected = Value::Regex(regex);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn block() {
		let block = Box::new(Block::default());
		let value = to_value(&block).unwrap();
		let expected = Value::Block(block);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn range() {
		let range = Box::new(Range {
			tb: "foo".to_owned(),
			beg: Bound::Included("foo".into()),
			end: Bound::Unbounded,
		});
		let value = to_value(&range).unwrap();
		let expected = Value::Range(range);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn edges() {
		let edges = Box::new(Edges {
			dir: Dir::In,
			from: sql::thing("foo:bar").unwrap(),
			what: Tables(vec!["foo".into()]),
		});
		let value = to_value(&edges).unwrap();
		let expected = Value::Edges(edges);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn future() {
		let future = Box::new(Future(Value::default().into()));
		let value = to_value(&future).unwrap();
		let expected = Value::Future(future);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());

		let future = Box::new(Future(Block(vec![Entry::Create(CreateStatement::default())])));
		let value = to_value(&future).unwrap();
		let expected = Value::Future(future);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn constant() {
		let constant = Constant::MathPi;
		let value = to_value(&constant).unwrap();
		let expected = Value::Constant(constant);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn function() {
		let function = Box::new(Function::Cast("foo".to_owned(), Default::default()));
		let value = to_value(&function).unwrap();
		let expected = Value::Function(function);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn subquery() {
		let subquery = Box::new(Subquery::Value(Value::None));
		let value = to_value(&subquery).unwrap();
		let expected = Value::Subquery(subquery);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn expression() {
		let expression = Box::new(Expression {
			l: "foo".into(),
			o: Operator::Equal,
			r: "Bar".into(),
		});
		let value = to_value(&expression).unwrap();
		let expected = Value::Expression(expression);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}

	#[test]
	fn unknown_struct() {
		#[derive(Debug, Serialize)]
		struct FooBar {
			foo: String,
			bar: i32,
		}

		let foo = "Foo";
		let bar = Default::default();
		let foo_bar = FooBar {
			bar,
			foo: foo.to_owned(),
		};
		let value = to_value(&foo_bar).unwrap();
		let expected = Value::Object(
			map! {
				"foo".to_owned() => foo.into(),
				"bar".to_owned() => bar.into(),
			}
			.into(),
		);
		assert_eq!(value, expected);
		assert_eq!(expected, to_value(&expected).unwrap());
	}
}