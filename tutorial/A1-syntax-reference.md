# 附录 A1 语法参考

## 完整语法 BNF

```
program        = form*
form           = expr
expr           = literal | symbol | keyword | list | vector | map | set
               | quote | syntax-quote | unquote | unquote-splice
literal        = integer | float | string | char | boolean | nil
integer        = -? [0-9]+
float          = -? [0-9]+ '.' [0-9]+ ([eE] [+-]? [0-9]+)?
string         = '"' ([^"\\] | '\\' .)* '"'
char           = '\' (char-name | .)
char-name      = 'newline' | 'space' | 'tab'
boolean        = 'true' | 'false'
nil            = 'nil'
keyword        = ':' ident
symbol         = ident
ident          = start-char rest-char*
list           = '(' expr* ')'
vector         = '[' expr* ']'
map            = '{' (expr expr)* '}'
set            = '#' '{' expr* '}'
quote          = "'" expr
syntax-quote   = '`' expr
unquote        = '~' expr
unquote-splice = '~@' expr
```

## 保留字

```
true false nil
def defn defn- def- defdata defdata-hit defpred defgeneric defmethod
defclass definstance defeffect defmacro defextern defglobal-type
defresource-algebra defprop defsession typefamily
fn let if cond match when unless
handle verify verify! check-equivalence find-attack
ns require use import refer
inline! specialize! opt-level suppress-warning
ann quote syntax-quote
fresh search solve-all find-all abduce constrain domain label
send recv close chan spawn
flat sharp shape crisp reflect-type gensym
```

## 运算符优先级

全部运算符为前缀（Lisp 风格），无中缀优先级问题。

## 常用特殊形式速查

| 形式 | 说明 | 示例 |
|------|------|------|
| `(defn name [params] body)` | 函数定义 | `(defn add [x y] (+ x y))` |
| `(defn- name [params] body)` | 私有函数 | `(defn- helper [x] (+ x 1))` |
| `(defdata (T a) (C1 ...) (C2 ...))` | ADT 定义 | `(defdata (Maybe a) (Nothing) (Just a))` |
| `(defdata-hit T ...)` | HIT 定义（⬜） | `(defdata-hit S1 (base) (loop [i : I] ...))` |
| `(defpred name [args] clauses...)` | 逻辑谓词 | `(defpred member [x y] ...)` |
| `(defgeneric name [args])` | 泛型函数 | `(defgeneric area [x])` |
| `(defmethod name mode [pat] body)` | 方法定义 | `(defmethod area [5] 50)` |
| `(defclass C [params] methods)` | 类型类 | `(defclass Coll [c e] (elem [c] -> e))` |
| `(definstance (C ...) methods)` | 类型类实例 | `(definstance (Coll i64 i64) (elem [x] x))` |
| `(defeffect E s (op [args] -> ret) ...)` | 效应声明 | `(defeffect State s (get [] -> s))` |
| `(defmacro name [args] body)` | 宏 | `(defmacro add1 [x] (+ x 1))` |
| `(defextern name "sym" "lib")` | FFI | `(defextern c-abs "abs" "libc.so.6")` |
| `(defprop name expr)` | 属性 | `(defprop p (model-check ...))` |
| `(typefamily F (T a) b ...)` | 类型族 | `(typefamily Elem (List a) a)` |
| `(rewrite F (T a) b)` | 类型族归约 | `(rewrite Elem (Map k v) k)` |
| `(defresource-algebra Name unit plus le)` | 资源代数 | `(defresource-algebra Cost 0 + <=)` |
| `(let [x e1 y e2] body)` | 局部绑定 | `(let [x 1] (+ x 1))` |
| `(if test then else)` | 条件 | `(if (> x 0) 1 0)` |
| `(cond t1 b1 t2 b2 default)` | 多分支 | `(cond (= x 0) "z" "nz")` |
| `(match v pat1 e1 pat2 e2 _ default)` | 模式匹配 | `(match m (Just x) x _ 0)` |
| `(handle body (E s) ops...)` | 效应处理 | `(handle (get) (State s) (get [] [k s] (k s s)))` |
| `(perform op args)` | 执行效应操作 | `(perform (get))` |
| `(fn [params] body)` | lambda | `(fn [x] (+ x 1))` |
| `(do e1 e2 e3)` | 顺序执行 | `(do (put 1) (get))` |
| `(ann expr Type)` | 类型标注 | `(ann 42 i64)` |
| `(comptime expr)` | 编译期求值 | `(comptime (+ 1 2))` |
| `(fresh [x y] body)` | 逻辑变量 | `(fresh [x] (domain x 1 5))` |
| `(search goal)` | 搜索 | `(search (member x xs))` |
| `(mlet [x e1] body)` | monadic let | `(mlet [x (get-m)] (pure x))` |
| `(get-m)` / `(put-m v)` / `(pure v)` | monadic 风格 | `(pure (get-m))` |
| `(pointcut fn params)` | AOP 切入点 | `(defaspect a (pointcut area [x]) ...)` |
| `(model-check init goal next depth)` | 模型检查 | `(model-check 0 (fn [n] ...) (fn [n] ...) 20)` |

## 类型与注解语法

| 注解 | 语法 | 示例 |
|------|------|------|
| 参数类型 | `[x : Type]` | `[x : i64]` |
| 返回类型 | `-> Type` | `-> i64` |
| 等级 + 类型 | `{grade x : Type}` | `{1 x : (Ptr a)}` |
| 精化类型 | `{x : T \| pred}` | `{n : i64 \| (>= n 0)}` |
| 六维注解 | `-> [effects, region, @grade, mode, det] Ret` | `-> [IO, rho1, @1, out, det] i64` |
| 依赖类型 | `Vec T n` / `Sigma n T` | `(Vec i64 n)` |
| 时序模态 | `⃝ T` / `□_t T` / `◇_t T` | `(⃝ (Stream a))` |
| 契约 | `:requires pred` / `:ensures pred` | `:requires (> d 0)` |

---

> 返回 [目录](INDEX.md)