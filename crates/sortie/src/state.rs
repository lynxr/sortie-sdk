//! Engine-stored state documents (`ctx.state::<T>("name")`), with compare-and-swap writes.

use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use sortie_proto::v1;

use crate::runtime::Session;
use crate::{Error, Result};

/// How many times [`StateDoc::update`] retries after a revision conflict.
pub const MAX_CAS_RETRIES: usize = 8;

/// A JSON document stored by the engine (SQLite), also read by the generator.
pub struct StateDoc<T> {
    session: Rc<Session>,
    scope: v1::StateScope,
    name: String,
    value: RefCell<T>,
    revision: Cell<u64>,
}

impl<T> std::fmt::Debug for StateDoc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateDoc")
            .field("name", &self.name)
            .field("revision", &self.revision.get())
            .finish()
    }
}

impl<T: Serialize + DeserializeOwned + Default + 'static> StateDoc<T> {
    pub(crate) async fn load(
        session: Rc<Session>,
        scope: v1::StateScope,
        name: &str,
    ) -> Result<Self> {
        let doc = Self {
            session,
            scope,
            name: name.to_owned(),
            value: RefCell::new(T::default()),
            revision: Cell::new(0),
        };
        doc.reload().await?;
        Ok(doc)
    }

    /// The local copy (as of the last load or successful write).
    pub fn get(&self) -> Ref<'_, T> {
        self.value.borrow()
    }

    /// Stored revision (0 = not stored yet).
    pub fn revision(&self) -> u64 {
        self.revision.get()
    }

    /// Re-reads the document from the engine.
    pub async fn reload(&self) -> Result<()> {
        let op = v1::state_op::Op::Get(v1::StateGet {
            scope: self.scope as i32,
            name: self.name.clone(),
        });
        let doc = self.expect_doc(self.session.state_op(op).await?)?;
        self.absorb(&doc)
    }

    /// Applies `f` and writes the result with compare-and-swap; on a conflict re-reads and
    /// re-applies `f` (up to [`MAX_CAS_RETRIES`] times).
    pub async fn update(&self, mut f: impl FnMut(&mut T)) -> Result<()> {
        for _ in 0..MAX_CAS_RETRIES {
            let mut next = serde_json::from_value::<T>(
                serde_json::to_value(&*self.value.borrow()).map_err(|e| self.err(e))?,
            )
            .map_err(|e| self.err(e))?;
            f(&mut next);
            let json = serde_json::to_vec(&next).map_err(|e| self.err(e))?;
            let op = v1::state_op::Op::Put(v1::StatePut {
                scope: self.scope as i32,
                name: self.name.clone(),
                expected_revision: self.revision.get(),
                json,
            });
            match self.session.state_op(op).await?.result {
                Some(v1::state_result::Result::Doc(doc)) => {
                    *self.value.borrow_mut() = next;
                    self.revision.set(doc.revision);
                    return Ok(());
                }
                Some(v1::state_result::Result::Err(e))
                    if e.code == v1::ErrorCode::Conflict as i32 =>
                {
                    self.reload().await?
                }
                Some(v1::state_result::Result::Err(e)) => return Err(self.err(e.message)),
                other => return Err(self.err(format!("unexpected reply {other:?}"))),
            }
        }
        Err(self.err(format!(
            "gave up after {MAX_CAS_RETRIES} conflicting writes"
        )))
    }

    /// Overwrites the document with `value` (still compare-and-swap on the current revision).
    pub async fn set(&self, value: T) -> Result<()> {
        let mut value = Some(value);
        self.update(move |v| {
            if let Some(new) = value.take() {
                *v = new;
            }
        })
        .await
    }

    fn expect_doc(&self, r: v1::StateResult) -> Result<v1::StateDoc> {
        match r.result {
            Some(v1::state_result::Result::Doc(doc)) => Ok(doc),
            Some(v1::state_result::Result::Err(e)) => Err(self.err(e.message)),
            other => Err(self.err(format!("unexpected reply {other:?}"))),
        }
    }

    fn absorb(&self, doc: &v1::StateDoc) -> Result<()> {
        let value = if doc.revision == 0 || doc.json.is_empty() {
            T::default()
        } else {
            serde_json::from_slice(&doc.json).map_err(|e| self.err(e))?
        };
        *self.value.borrow_mut() = value;
        self.revision.set(doc.revision);
        Ok(())
    }

    fn err(&self, e: impl std::fmt::Display) -> Error {
        Error::State {
            name: self.name.clone(),
            message: e.to_string(),
        }
    }
}
