use rspack_cacheable::{cacheable, cacheable_dyn};
use rspack_hash::{RspackHash, RspackHashable};

use crate::{
  Compilation, DependencyCodeGeneration, DependencyRange, DependencyTemplate,
  DependencyTemplateType, RuntimeGlobals, RuntimeSpec, TemplateContext, TemplateReplaceSource,
};

#[cacheable]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum RuntimeRequirementsDependencyMode {
  #[default]
  Normal,
  Call,
  AddOnly,
}

impl RspackHashable for RuntimeRequirementsDependencyMode {
  fn hash(&self, state: &mut RspackHash) {
    match self {
      RuntimeRequirementsDependencyMode::Normal => "normal",
      RuntimeRequirementsDependencyMode::Call => "call",
      RuntimeRequirementsDependencyMode::AddOnly => "add-only",
    }
    .hash(state);
  }
}

#[cacheable]
#[derive(Debug, Clone)]
pub struct RuntimeRequirementsDependency {
  pub range: DependencyRange,
  pub runtime_requirements: RuntimeGlobals,
  pub mode: RuntimeRequirementsDependencyMode,
}

#[cacheable_dyn]
impl DependencyCodeGeneration for RuntimeRequirementsDependency {
  fn dependency_template(&self) -> Option<DependencyTemplateType> {
    Some(RuntimeRequirementsDependencyTemplate::template_type())
  }

  fn update_hash(
    &self,
    hasher: &mut RspackHash,
    _compilation: &Compilation,
    _runtime: Option<&RuntimeSpec>,
  ) {
    self.range.hash(hasher);
    self.runtime_requirements.hash(hasher);
    self.mode.hash(hasher);
  }
}

impl RuntimeRequirementsDependency {
  pub fn new(range: DependencyRange, runtime_requirements: RuntimeGlobals) -> Self {
    Self {
      range,
      runtime_requirements,
      mode: RuntimeRequirementsDependencyMode::Normal,
    }
  }
  pub fn call(range: DependencyRange, runtime_requirements: RuntimeGlobals) -> Self {
    Self {
      range,
      runtime_requirements,
      mode: RuntimeRequirementsDependencyMode::Call,
    }
  }
  pub fn add_only(runtime_requirements: RuntimeGlobals) -> Self {
    Self {
      range: DependencyRange::default(),
      runtime_requirements,
      mode: RuntimeRequirementsDependencyMode::AddOnly,
    }
  }
}

#[cacheable]
#[derive(Debug, Clone, Default)]
pub struct RuntimeRequirementsDependencyTemplate;

impl RuntimeRequirementsDependencyTemplate {
  pub fn template_type() -> DependencyTemplateType {
    DependencyTemplateType::Custom("RuntimeRequirementsDependency")
  }
}

impl DependencyTemplate for RuntimeRequirementsDependencyTemplate {
  fn render(
    &self,
    dep: &dyn DependencyCodeGeneration,
    source: &mut TemplateReplaceSource,
    code_generatable_context: &mut TemplateContext,
  ) {
    let dep = dep
      .as_any()
      .downcast_ref::<RuntimeRequirementsDependency>()
      .expect(
        "RuntimeRequirementsDependencyTemplate should be used for RuntimeRequirementsDependency",
      );

    if matches!(dep.mode, RuntimeRequirementsDependencyMode::AddOnly) {
      code_generatable_context
        .runtime_template
        .runtime_requirements_mut()
        .insert(dep.runtime_requirements);
      return;
    }

    let mut content = code_generatable_context
      .runtime_template
      .render_runtime_globals(&dep.runtime_requirements);

    if matches!(dep.mode, RuntimeRequirementsDependencyMode::Call) {
      content = format!("{content}()");
    }

    source.replace(dep.range.start, dep.range.end, content, None);
  }
}
