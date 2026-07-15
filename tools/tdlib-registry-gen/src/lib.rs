//! Offline-генератор полного registry закреплённой TDLib-схемы.
//!
//! Наполняется в фазе P3 (`plans.md`): generated registry всех 1010 methods
//! поверх parser из `telegram_core::schema` плюс capability-таблица с
//! default-deny для неотревьюенных методов (`docs/capability-notes.md`).

#![forbid(unsafe_code)]
