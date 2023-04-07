#[cfg(feature = "program")]
use solana_program::{program_error::ProgramError, program_pack::Sealed};

#[cfg(feature = "program")]
pub trait Padded: Sealed {
    fn get_padding_mut(&mut self) -> Result<(usize, &mut Box<Vec<u8>>), ProgramError>;

    fn update_padding(&mut self) -> Result<(), ProgramError> {
        let (needed_size, padding) = self.get_padding_mut()?;
        let current_size = padding.len();
        if needed_size < current_size {
            // Registration
            padding.truncate(needed_size);
        } else if needed_size > current_size {
            // Initialization or unregistration
            padding.resize(needed_size, 0u8);
        }
        Ok(())
    }
}
