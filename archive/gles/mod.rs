//! Optional Smithay/GLES rendering. The host owns its renderer and output scheduling.
mod ids;
pub mod text;
pub use text::{PreparedUiText, UiTextElement, UiTextRenderer};
pub mod card;
pub use card::{CardRenderer, LabelRenderElement, OverlayCardStyle};
mod view;
pub use view::{UiRenderElement, UiRenderer};
