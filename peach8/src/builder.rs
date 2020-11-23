use crate::context::Context;
use crate::peach::Peach8;

pub struct Builder<'a, C: Context> {
    context: Option<C>,
    program: Option<&'a [u8]>,
}

impl<'a, C: Context> Builder<'a, C> {
    pub fn new() -> Self {
        Self {
            context: None,
            program: None,
        }
    }

    pub fn with_context(mut self, ctx: C) -> Self {
        self.context = Some(ctx);
        self
    }

    pub fn with_program(mut self, prog: &'a [u8]) -> Self {
        self.program = Some(prog);
        self
    }

    pub fn build(self) -> Result<Peach8<C>, &'static str> {
        let context = self.context.ok_or("Context not provided")?;
        let program = self.program.ok_or("Program not provided")?;
        let mut peach = Peach8::new(context);
        peach.load(program);
        Ok(peach)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::testing::TestingContext;

    #[test]
    fn with_context_and_prog() {
        let result = Builder::new()
            .with_context(TestingContext::new(0))
            .with_program(&[])
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn with_context_only() {
        let result = Builder::new().with_context(TestingContext::new(0)).build();
        assert!(result.is_err());
    }

    #[test]
    fn with_program_only() {
        let result = Builder::<'_, TestingContext>::new()
            .with_program(&[])
            .build();
        assert!(result.is_err());
    }
}
