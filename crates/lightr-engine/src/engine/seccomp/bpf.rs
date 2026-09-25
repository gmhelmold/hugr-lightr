use super::{
    err_unsupported, jump, stmt, BPF_ABS, BPF_ALU, BPF_AND, BPF_JA, BPF_JEQ, BPF_JGE, BPF_JGT,
    BPF_JMP, BPF_JSET, BPF_K, BPF_LD, BPF_RET, BPF_W,
};

#[derive(Clone, Copy)]
pub(super) enum ArgOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    MaskedEq,
}

#[derive(Clone, Copy)]
pub(super) struct ArgCondition {
    pub(super) low_offset: u32,
    pub(super) high_offset: u32,
    pub(super) value_low: u32,
    pub(super) value_high: u32,
    pub(super) value_two_low: u32,
    pub(super) value_two_high: u32,
    pub(super) op: ArgOp,
}

pub(super) struct Rule {
    pub(super) nr: u32,
    pub(super) action: u32,
    pub(super) args: Vec<ArgCondition>,
}

#[derive(Clone, Copy)]
struct Label(usize);

struct BpfBuilder {
    prog: Vec<libc::sock_filter>,
    labels: Vec<Option<usize>>,
    patches: Vec<(usize, Label)>,
}

impl BpfBuilder {
    fn new() -> Self {
        Self {
            prog: Vec::new(),
            labels: Vec::new(),
            patches: Vec::new(),
        }
    }

    fn label(&mut self) -> Label {
        let label = Label(self.labels.len());
        self.labels.push(None);
        label
    }

    fn bind(&mut self, label: Label) {
        self.labels[label.0] = Some(self.prog.len());
    }

    fn push(&mut self, insn: libc::sock_filter) {
        self.prog.push(insn);
    }

    fn jump_to(&mut self, label: Label) {
        let at = self.prog.len();
        self.push(stmt(BPF_JMP | BPF_JA, 0));
        self.patches.push((at, label));
    }

    fn finish(mut self) -> std::io::Result<Vec<libc::sock_filter>> {
        for (at, label) in self.patches {
            let target = self.labels[label.0]
                .ok_or_else(|| err_unsupported("internal seccomp label was not bound"))?;
            let offset = target
                .checked_sub(at + 1)
                .ok_or_else(|| err_unsupported("internal seccomp jump underflow"))?;
            self.prog[at].k = u32::try_from(offset)
                .map_err(|_| err_unsupported("internal seccomp jump offset overflow"))?;
        }
        Ok(self.prog)
    }
}

pub(super) fn compile(default_ret: u32, rules: &[Rule]) -> std::io::Result<Vec<libc::sock_filter>> {
    // Conditional jumps use only local jt/jf values. Long skips and all
    // forward references use JA, whose offset is a u32, so large profiles do
    // not silently truncate an 8-bit conditional offset.
    let mut b = BpfBuilder::new();
    b.push(stmt(
        BPF_LD | BPF_W | BPF_ABS,
        super::SECCOMP_DATA_ARCH_OFFSET,
    ));
    b.push(jump(
        BPF_JMP | BPF_JEQ | BPF_K,
        super::AUDIT_ARCH_X86_64,
        1,
        0,
    ));
    b.push(stmt(BPF_RET | BPF_K, super::SECCOMP_RET_KILL_PROCESS));
    b.push(stmt(
        BPF_LD | BPF_W | BPF_ABS,
        super::SECCOMP_DATA_NR_OFFSET,
    ));
    // x32 reports x86_64 audit arch but sets this syscall-number bit. Its
    // numbers are not in native table, so default-ALLOW would bypass rules.
    b.push(jump(
        BPF_JMP | BPF_JSET | BPF_K,
        super::X32_SYSCALL_BIT,
        0,
        1,
    ));
    b.push(stmt(BPF_RET | BPF_K, super::SECCOMP_RET_KILL_PROCESS));

    let rule_labels: Vec<_> = (0..=rules.len()).map(|_| b.label()).collect();
    for (index, rule) in rules.iter().enumerate() {
        b.bind(rule_labels[index]);
        // Argument tests overwrite the accumulator, so every rule reloads the
        // syscall number before comparing it.
        b.push(stmt(
            BPF_LD | BPF_W | BPF_ABS,
            super::SECCOMP_DATA_NR_OFFSET,
        ));
        // Match → skip the failure jump; no match → jump to the next rule.
        b.push(jump(BPF_JMP | BPF_JEQ | BPF_K, rule.nr, 1, 0));
        b.jump_to(rule_labels[index + 1]);
        for arg in &rule.args {
            let next = b.label();
            emit_condition(&mut b, *arg, next, rule_labels[index + 1]);
            b.bind(next);
        }
        b.push(stmt(BPF_RET | BPF_K, rule.action));
    }
    b.bind(rule_labels[rules.len()]);
    b.push(stmt(BPF_RET | BPF_K, default_ret));
    b.finish()
}

fn emit_condition(b: &mut BpfBuilder, arg: ArgCondition, next: Label, fail: Label) {
    match arg.op {
        ArgOp::Eq => {
            emit_eq_word(b, arg.high_offset, arg.value_high, fail);
            emit_eq_word(b, arg.low_offset, arg.value_low, fail);
        }
        ArgOp::Ne => {
            // A 64-bit value differs when either half differs. Equal high half
            // falls through to low; a differing high half passes immediately.
            b.push(stmt(BPF_LD | BPF_W | BPF_ABS, arg.high_offset));
            b.push(jump(BPF_JMP | BPF_JEQ | BPF_K, arg.value_high, 1, 0));
            b.jump_to(next);
            b.push(stmt(BPF_LD | BPF_W | BPF_ABS, arg.low_offset));
            b.push(jump(BPF_JMP | BPF_JEQ | BPF_K, arg.value_low, 0, 1));
            b.jump_to(fail);
        }
        ArgOp::MaskedEq => {
            emit_masked_eq_word(b, arg.high_offset, arg.value_high, arg.value_two_high, fail);
            emit_masked_eq_word(b, arg.low_offset, arg.value_low, arg.value_two_low, fail);
        }
        ArgOp::Lt | ArgOp::Le | ArgOp::Gt | ArgOp::Ge => {
            emit_ordered(b, arg, next, fail);
        }
    }
}

fn emit_eq_word(b: &mut BpfBuilder, offset: u32, value: u32, fail: Label) {
    b.push(stmt(BPF_LD | BPF_W | BPF_ABS, offset));
    b.push(jump(BPF_JMP | BPF_JEQ | BPF_K, value, 1, 0));
    b.jump_to(fail);
}

fn emit_masked_eq_word(b: &mut BpfBuilder, offset: u32, value: u32, mask: u32, fail: Label) {
    b.push(stmt(BPF_LD | BPF_W | BPF_ABS, offset));
    b.push(stmt(BPF_ALU | BPF_AND | BPF_K, mask));
    b.push(jump(BPF_JMP | BPF_JEQ | BPF_K, value & mask, 1, 0));
    b.jump_to(fail);
}

fn emit_ordered(b: &mut BpfBuilder, arg: ArgCondition, next: Label, fail: Label) {
    let high_greater_passes = matches!(arg.op, ArgOp::Gt | ArgOp::Ge);
    let low_fail_op = match arg.op {
        ArgOp::Lt => BPF_JGE,
        ArgOp::Le => BPF_JGT,
        ArgOp::Gt => BPF_JGT,
        ArgOp::Ge => BPF_JGE,
        _ => unreachable!(),
    };
    let low_true_passes = matches!(arg.op, ArgOp::Gt | ArgOp::Ge);

    // High-word > value: relation is already decided. High-word < value is
    // likewise decided by the equal check below; equal falls to low word.
    b.push(stmt(BPF_LD | BPF_W | BPF_ABS, arg.high_offset));
    b.push(jump(BPF_JMP | BPF_JGT | BPF_K, arg.value_high, 0, 1));
    if high_greater_passes {
        b.jump_to(next);
    } else {
        b.jump_to(fail);
    }
    b.push(jump(BPF_JMP | BPF_JEQ | BPF_K, arg.value_high, 1, 0));
    if high_greater_passes {
        b.jump_to(fail);
    } else {
        b.jump_to(next);
    }

    // For equal high words, the same relation is evaluated on low words.
    b.push(stmt(BPF_LD | BPF_W | BPF_ABS, arg.low_offset));
    b.push(jump(
        BPF_JMP | low_fail_op | BPF_K,
        arg.value_low,
        if low_true_passes { 1 } else { 0 },
        if low_true_passes { 0 } else { 1 },
    ));
    b.jump_to(fail);
}
