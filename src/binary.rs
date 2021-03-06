use goblin::elf::header::Header as ElfHeader;
use goblin::elf::header::ET_DYN;
use goblin::elf::header::ET_EXEC;
use goblin::elf::Elf;

use std::path::Path;
use std::path::PathBuf;

use crate::errors::Runner::ArgError;
use crate::errors::SolverError;
use crate::errors::SolverResult;

#[derive(Debug)]
pub struct Binary {
    path: PathBuf,
    elf_header: ElfHeader,
}

/// holds the path to the binary and its ELF header
impl Binary {
    pub fn new(path: &Path) -> SolverResult<Binary> {
        let elf_header = {
            let bytes = std::fs::read(path)?;
            let elf = Elf::parse(&bytes)?;
            elf.header
        };

        Ok(Binary {
            path: path.to_path_buf(),
            elf_header,
        })
    }

    /// Determine whether the binary is a position independent executable (PIE)
    pub fn is_pie(&self) -> SolverResult<bool> {
        match self.elf_header.e_type {
            ET_DYN => Ok(true),
            ET_EXEC => Ok(false),
            _ => Err(SolverError::new(
                ArgError,
                "`e_type` is not `ET_DYN` or `ET_EXEC`",
            )),
        }
    }
}
