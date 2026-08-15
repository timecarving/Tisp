#!/usr/bin/env bash
# Tisp 范式可用性验收矩阵(OpenSpec complete-declarative-paradigms-aop)
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${BIN:-$ROOT/target/debug/tisp}"
if [ ! -x "$BIN" ]; then
  echo "missing $BIN; run: cargo build -p tisp-cli" >&2
  exit 1
fi
fail=0
check() {
  local desc="$1"; shift
  local expect="$1"; shift
  local err ec
  "$@" >/tmp/tisp-matrix.out 2>/tmp/tisp-matrix.err
  ec=$?
  err="$(cat /tmp/tisp-matrix.err)"
  if [ "$expect" = pass ] && [ $ec -ne 0 ]; then
    echo "FAIL $desc (exit $ec): $err"; fail=1
  elif [ "$expect" = fail ] && [ $ec -eq 0 ]; then
    echo "FAIL $desc (应失败但通过)"; fail=1
  else
    echo "PASS $desc"
  fi
}

# 8 编程范式(纯声明式副作用管理)
check "8 范式 typecheck"            pass "$BIN" --typecheck "$ROOT/examples/declarative-paradigms.tisp"
check "8 范式 run"                  pass "$BIN" --run "$ROOT/examples/declarative-paradigms.tisp"
check "范式矩阵 typecheck"          pass "$BIN" --typecheck "$ROOT/examples/paradigm-matrix.tisp"
check "范式矩阵 run"                pass "$BIN" --run "$ROOT/examples/paradigm-matrix.tisp"
# comptime + MOP + AOP
check "AOP/MOP typecheck"           pass "$BIN" --typecheck "$ROOT/examples/aop-mop.tisp"
check "AOP/MOP run"                 pass "$BIN" --run "$ROOT/examples/aop-mop.tisp"
check "comptime 内联 --desugar"     pass "$BIN" --desugar /dev/stdin <<< '(defn main [] (+ (comptime (+ 1 2)) 4))'
check "comptime 编译期错误"         fail "$BIN" --typecheck /dev/stdin <<< '(defn main [] (comptime (no-such-fn 1)))'
# 静态类型 + 纯声明 + 统一内存约束
check "--eval 求值"                 pass "$BIN" --eval '(+ 1 2)'
check "类型错误 --run 拒绝"         fail "$BIN" --run /dev/stdin <<< '(defn main [] (+ 1 true))'
check "Unsafe 门控"                 fail "$BIN" --typecheck /dev/stdin <<< '(defn main [] (ptr-read 1))'
check "State 门控"                  fail "$BIN" --typecheck /dev/stdin <<< '(defn main [] (stack-peek (stack-push (stack-new) 1)))'
check "液态违反拒绝"                fail "$BIN" --typecheck "$ROOT/examples/liquid-types-violations.tisp"
check "线性句柄复用拒绝"            fail "$BIN" --typecheck /dev/stdin <<< '(defn bad [{1 c : (Chan i64)}] (do (async-send c 1) (async-send c 2)))'

rm -f /tmp/tisp-matrix.out /tmp/tisp-matrix.err
if [ $fail -ne 0 ]; then
  echo "矩阵存在失败项"
  exit 1
fi
echo "全部范式可用性检查通过"
