from math import abs, max, min, clamp, pow

# 1. Test standard library math generic functions
print("--- Standard Math Library Generics ---")
print("abs(-5.5) =")
print(abs<float>(-5.5))

print("abs(-10) =")
print(abs<int>(-10))

print("max(2.5, 4.2) =")
print(max<float>(2.5, 4.2))

print("min(10, 2) =")
print(min<int>(10, 2))

print("clamp(15.0, 0.0, 10.0) =")
print(clamp<float>(15.0, 0.0, 10.0))

print("pow(2.0, 3) =")
print(pow<float>(2.0, 3))

print("pow(3, 4) =")
print(pow<int>(3, 4))

# 2. Test Matrix elementwise operations
print("\n--- Matrix Elementwise Ops ---")
let m1 = Matrix(2, 2)
m1.set(0, 0, 1.0)
m1.set(0, 1, 2.0)
m1.set(1, 0, 3.0)
m1.set(1, 1, 4.0)

let m2 = Matrix(2, 2)
m2.set(0, 0, 0.5)
m2.set(0, 1, 1.5)
m2.set(1, 0, 2.5)
m2.set(1, 1, 3.5)

let m_sub = matrix_sub(m1, m2)
print("m1 - m2 =")
print(m_sub.get(0, 0))
print(m_sub.get(0, 1))
print(m_sub.get(1, 0))
print(m_sub.get(1, 1))

let m_scale = matrix_scale(m1, 2.0)
print("m1 * 2 =")
print(m_scale.get(0, 0))
print(m_scale.get(0, 1))

let m_sig = matrix_sigmoid(m1)
print("sigmoid(m1) [0,0]:")
print(m_sig.get(0, 0))

let m_exp = matrix_exp(m1)
print("exp(m1) [0,0]:")
print(m_exp.get(0, 0))

let m_log = matrix_log(m1)
print("log(m1) [0,0]:")
print(m_log.get(0, 0))


# 3. Test Tensors and broadcasting
print("\n--- Tensors and Broadcasting ---")
let t1 = Tensor([2, 3])
t1[(0, 0)] = 1.0
t1[(0, 1)] = 2.0
t1[(0, 2)] = 3.0
t1[(1, 0)] = 4.0
t1[(1, 1)] = 5.0
t1[(1, 2)] = 6.0

let t2 = Tensor([1, 3])
t2[(0, 0)] = 1.0
t2[(0, 1)] = 0.0
t2[(0, 2)] = 2.0

let t_sum = t1 + t2
print("t1 + t2 =")
print(t_sum[(0, 0)]) # 1.0 + 1.0 = 2.0
print(t_sum[(0, 1)]) # 2.0 + 0.0 = 2.0
print(t_sum[(0, 2)]) # 3.0 + 2.0 = 5.0
print(t_sum[(1, 0)]) # 4.0 + 1.0 = 5.0
print(t_sum[(1, 1)]) # 5.0 + 0.0 = 5.0
print(t_sum[(1, 2)]) # 6.0 + 2.0 = 8.0

let t_sub = t1 - t2
print("t1 - t2 [1, 2] =")
print(t_sub[(1, 2)]) # 6.0 - 2.0 = 4.0

let t_sig = t1.sigmoid()
print("sigmoid(t1) [0, 0] =")
print(t_sig[(0, 0)])

let t_relu = t1.relu()
print("relu(t1) [0, 0] =")
print(t_relu[(0, 0)])

print("Finished!")
