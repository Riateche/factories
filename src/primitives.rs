use {
    crate::rf,
    derive_more::{From, Into},
    once_cell::sync::Lazy,
    ordered_float::OrderedFloat,
    serde::{Deserialize, Serialize},
    std::{
        fmt::{self, Display, Formatter},
        iter::Sum,
        ops::{Mul, Neg, Sub, SubAssign},
        str::FromStr,
    },
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct Speed(OrderedFloat<f64>);

impl Speed {
    pub const ZERO: Self = Self(OrderedFloat(0.0));
    pub const ONE: Self = Self(OrderedFloat(1.0));
}

impl From<f64> for Speed {
    fn from(value: f64) -> Self {
        Self(value.into())
    }
}

impl From<Speed> for f64 {
    fn from(value: Speed) -> Self {
        value.0.into()
    }
}

impl FromStr for Speed {
    type Err = <f64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(f64::from_str(s)?.into())
    }
}

impl Display for Speed {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}/s", rf((*self).into()))
    }
}

impl Neg for Speed {
    type Output = Speed;

    fn neg(self) -> Self::Output {
        (-f64::from(self)).into()
    }
}

impl Sum<Speed> for Speed {
    fn sum<I: Iterator<Item = Speed>>(iter: I) -> Self {
        iter.map(f64::from).sum::<f64>().into()
    }
}

impl SubAssign for Speed {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 .0 -= rhs.0 .0;
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct MachineCount(OrderedFloat<f64>);

impl From<f64> for MachineCount {
    fn from(value: f64) -> Self {
        Self(value.into())
    }
}

impl From<MachineCount> for f64 {
    fn from(value: MachineCount) -> Self {
        value.0.into()
    }
}

impl FromStr for MachineCount {
    type Err = <f64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(f64::from_str(s)?.into())
    }
}

impl Display for MachineCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", rf((*self).into()))
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    From,
    Into,
    Serialize,
    Deserialize,
)]
pub struct Amount(OrderedFloat<f64>);

impl Amount {
    pub const ZERO: Self = Self(OrderedFloat(0.0));
    pub const ONE: Self = Self(OrderedFloat(1.0));
}

impl From<f64> for Amount {
    fn from(value: f64) -> Self {
        Self(value.into())
    }
}

impl From<Amount> for f64 {
    fn from(value: Amount) -> Self {
        value.0.into()
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", rf((*self).into()))
    }
}

impl Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Self) -> Self::Output {
        (f64::from(self) - f64::from(rhs)).into()
    }
}

impl Mul<Speed> for Amount {
    type Output = Speed;

    fn mul(self, rhs: Speed) -> Self::Output {
        (f64::from(self) * f64::from(rhs)).into()
    }
}

impl Mul<Amount> for Speed {
    type Output = Speed;

    fn mul(self, rhs: Amount) -> Self::Output {
        (f64::from(self) * f64::from(rhs)).into()
    }
}

impl Mul<f64> for Speed {
    type Output = Speed;

    fn mul(self, rhs: f64) -> Self::Output {
        (f64::from(self) * rhs).into()
    }
}

impl Mul<Speed> for f64 {
    type Output = Speed;

    fn mul(self, rhs: Speed) -> Self::Output {
        (self * f64::from(rhs)).into()
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct ItemName(String);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct RecipeName(String);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct CrafterName(String);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct ModuleName(String);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct RecipeCategory(String);

impl Display for ItemName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for RecipeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for CrafterName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for ModuleName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for RecipeCategory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<&str> for ItemName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}
impl PartialEq<&str> for RecipeCategory {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl ItemName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl ModuleName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl CrafterName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl RecipeName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl From<&str> for ItemName {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<&str> for ModuleName {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<&str> for CrafterName {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<&str> for RecipeName {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<&str> for RecipeCategory {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl CrafterName {
    pub const SOURCE: Lazy<Self> = Lazy::new(|| "source".into());
    pub const SINK: Lazy<Self> = Lazy::new(|| "sink".into());
}
impl RecipeCategory {
    pub const SOURCE: Lazy<Self> = Lazy::new(|| "source".into());
    pub const SINK: Lazy<Self> = Lazy::new(|| "sink".into());
}
