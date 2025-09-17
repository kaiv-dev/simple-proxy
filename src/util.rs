#![allow(non_snake_case)]
#![allow(unused)]

use std::str::FromStr;

#[derive(Debug)]
pub enum ParseError {
    Missing,
    Invalid,
}

impl ParseError {
    pub fn describe_panic(&self, name: &'static str, ty: &'static str) -> ! {
        match self {
            Self::Invalid => panic!("Invalid env var: {} - must be {}", name, ty),
            Self::Missing => panic!("Missing required env var: {}", name)
        }
    }
}

pub trait TryParse<E> {
    fn try_parse<T : std::str::FromStr>(self) -> Result<T, E>;
}

impl<E> TryParse<ParseError> for Result<String, E> {
    fn try_parse<T: std::str::FromStr>(self) -> Result<T, ParseError> {
        match self {
            Ok(v) => v.parse::<T>().ok().ok_or(ParseError::Invalid),
            Err(_) => Err(ParseError::Missing),
        }
    }
}



pub trait Operator<T, E> {
    fn if_none(self, rh: Result<T, E>) -> Result<T, E>;
}

impl<T ,E> Operator<T, E> for () {
    fn if_none(self, rh: Result<T, E>) -> Result<T, E> {
        rh 
    }
}

impl<T> Operator<T, ParseError> for (T,) {
    fn if_none(self, rh: Result<T, ParseError>) -> Result<T, ParseError> {
        match rh {
            Ok(v) => Ok(v),
            Err(_e) => Ok(self.0),
        }
    }
}



#[macro_export]
macro_rules! env_config {
    ($($filename:expr => $glob:ident = $struct:ident {$($field:ident : $type:ty $(= $op_val:expr)? ),* $(,)?})*) => {
        $(
            #[allow(non_snake_case)]
            pub(crate) struct $struct {
                $(pub $field: $type),*
            }
            impl $struct {
                fn new() -> Self {
                    Self {
                        $(
                            $field: 
                            $crate::util::Operator::if_none(($($op_val,)?), 
                            $crate::util::TryParse::try_parse::<$type>(std::env::var(stringify!($field).to_ascii_uppercase()))
                            ).unwrap_or_else(|e| e.describe_panic(stringify!($field), stringify!($type))),
                        )*
                    }
                }
            }

            pub(crate) static $glob : once_cell::sync::Lazy<$struct> = once_cell::sync::Lazy::new(|| {
                dotenvy::from_filename_override($filename).ok(); // only for develop
                $struct::new()
            });
        )*
    };
    ($($filename:expr => pub $glob:ident = $struct:ident {$($field:ident : $type:ty $(= $op_val:expr)? ),* $(,)?})*) => {
        $(
            #[allow(non_snake_case)]
            pub struct $struct {
                $(pub $field: $type),*
            }
            impl $struct {
                fn new() -> Self {
                    Self {
                        $(
                            $field: 
                            $crate::utils::env::Operator::if_none(($($op_val,)?), 
                            $crate::utils::env::TryParse::try_parse::<$type>(std::env::var(stringify!($field).to_ascii_uppercase()))
                            ).unwrap_or_else(|e| e.describe_panic(stringify!($field), stringify!($type))),
                        )*
                    }
                }
            }

            pub static $glob : once_cell::sync::Lazy<$struct> = once_cell::sync::Lazy::new(|| {
                dotenvy::from_filename_override($filename).ok(); // only for develop
                $struct::new()
            });
        )*
    };
}


#[macro_export]
macro_rules! wrap {
    ($name:ident($ty:ty) $(= $($tr:ty),* $(,)?)?) => {
        $(#[derive($($tr),*)])?
        struct $name($ty);
        $crate::wrap!(@$name($ty));
    };
    ($name:ident(pub $ty:ty) $(= $($tr:ty),* $(,)?)?) => {
        $(#[derive($($tr),*)])?
        struct $name(pub $ty);
        $crate::wrap!(@$name($ty));
    };
    (pub $name:ident($ty:ty) $(= $($tr:ty),* $(,)?)?) => {
        $(#[derive($($tr),*)])?
        pub struct $name($ty);
        $crate::wrap!(@$name($ty));
    };
    (pub $name:ident(pub $ty:ty) $(= $($tr:ty),* $(,)?)?) => {
        $(#[derive($($tr),*)])?
        pub struct $name(pub $ty);
        $crate::wrap!(@$name($ty));
    };
    (@$name:ident($ty:ty)) => {
        impl std::ops::Deref for $name {
            type Target = $ty;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut $ty {
                &mut self.0
            }
        }
    };
}
