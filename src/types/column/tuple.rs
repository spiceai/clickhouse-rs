use std::sync::Arc;

use chrono_tz::Tz;

use crate::{
    binary::{Encoder, ReadEx},
    errors::Result,
    types::{
        column::{
            column_data::{ArcColumnData, BoxColumnData},
            ArcColumnWrapper, ColumnData,
        },
        SqlType, Value, ValueRef,
    },
};

/// A `Tuple` column: one column per element, each holding every row's value for that element.
pub(crate) struct TupleColumnData {
    pub(crate) names: Vec<Option<String>>,
    pub(crate) inner: Vec<ArcColumnData>,
}

impl TupleColumnData {
    pub(crate) fn load<R: ReadEx>(
        reader: &mut R,
        elements: Vec<(Option<&str>, &str)>,
        rows: usize,
        tz: Tz,
    ) -> Result<Self> {
        let mut names = Vec::with_capacity(elements.len());
        let mut inner = Vec::with_capacity(elements.len());
        for (name, type_name) in elements {
            names.push(name.map(str::to_string));
            inner.push(<dyn ColumnData>::load_data::<ArcColumnWrapper, _>(
                reader, type_name, rows, tz,
            )?);
        }
        Ok(Self { names, inner })
    }
}

impl ColumnData for TupleColumnData {
    fn sql_type(&self) -> SqlType {
        SqlType::Tuple(
            self.names
                .iter()
                .zip(&self.inner)
                .map(|(name, column)| (name.clone(), column.sql_type().into()))
                .collect(),
        )
    }

    fn save(&self, encoder: &mut Encoder, start: usize, end: usize) {
        for column in &self.inner {
            column.save(encoder, start, end);
        }
    }

    fn len(&self) -> usize {
        self.inner.first().map_or(0, |column| column.len())
    }

    fn push(&mut self, value: Value) {
        if let Value::Tuple(vs) = value {
            assert_eq!(vs.len(), self.inner.len(), "tuple arity mismatch");
            for (column, v) in self.inner.iter_mut().zip(vs.iter()) {
                Arc::get_mut(column).unwrap().push(v.clone());
            }
        } else {
            panic!("value should be a tuple")
        }
    }

    fn at(&self, index: usize) -> ValueRef<'_> {
        ValueRef::Tuple(Arc::new(
            self.inner.iter().map(|column| column.at(index)).collect(),
        ))
    }

    fn clone_instance(&self) -> BoxColumnData {
        Box::new(Self {
            names: self.names.clone(),
            inner: self.inner.clone(),
        })
    }

    fn get_timezone(&self) -> Option<Tz> {
        None
    }
}
