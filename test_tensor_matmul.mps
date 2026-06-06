# test_tensor_matmul.mps

# 1. Test creation functions
print("--- Testing Creation Functions ---")
let tz = tensor_zeros([2, 3])
let to = tensor_ones([2, 3])
print("zeros[0, 0] =")
print(tz[(0, 0)])  # should be 0.0
print("ones[0, 0] =")
print(to[(0, 0)])  # should be 1.0

let tz32 = tensor32_zeros([2, 3])
let to32 = tensor32_ones([2, 3])
print("zeros32[0, 0] =")
print(tz32[(0, 0)])  # should be 0.0
print("ones32[0, 0] =")
print(to32[(0, 0)])  # should be 1.0

# 2. Test `@` operator on Matrix (2D)
print("--- Testing Matrix @ ---")
let m1 = Matrix(2, 3)
m1.set(0, 0, 1.0)
m1.set(0, 1, 2.0)
m1.set(0, 2, 3.0)
m1.set(1, 0, 4.0)
m1.set(1, 1, 5.0)
m1.set(1, 2, 6.0)

let m2 = Matrix(3, 2)
m2.set(0, 0, 7.0)
m2.set(0, 1, 8.0)
m2.set(1, 0, 9.0)
m2.set(1, 1, 10.0)
m2.set(2, 0, 11.0)
m2.set(2, 1, 12.0)

let m_res = m1 @ m2
print("m_res[0, 0] =")  # 1*7 + 2*9 + 3*11 = 7 + 18 + 33 = 58
print(m_res.get(0, 0))
print("m_res[0, 1] =")  # 1*8 + 2*10 + 3*12 = 8 + 20 + 36 = 64
print(m_res.get(0, 1))
print("m_res[1, 0] =")  # 4*7 + 5*9 + 6*11 = 28 + 45 + 66 = 139
print(m_res.get(1, 0))
print("m_res[1, 1] =")  # 4*8 + 5*10 + 6*12 = 32 + 50 + 72 = 154
print(m_res.get(1, 1))

# 3. Test `@` operator on Tensor (2D)
print("--- Testing Tensor @ ---")
let t1 = Tensor([2, 3])
t1[(0, 0)] = 1.0
t1[(0, 1)] = 2.0
t1[(0, 2)] = 3.0
t1[(1, 0)] = 4.0
t1[(1, 1)] = 5.0
t1[(1, 2)] = 6.0

let t2 = Tensor([3, 2])
t2[(0, 0)] = 7.0
t2[(0, 1)] = 8.0
t2[(1, 0)] = 9.0
t2[(1, 1)] = 10.0
t2[(2, 0)] = 11.0
t2[(2, 1)] = 12.0

let t_res = t1 @ t2
print("t_res[0, 0] =")  # 58.0
print(t_res[(0, 0)])
print("t_res[1, 1] =")  # 154.0
print(t_res[(1, 1)])

# 4. Test batched matmul and broadcasting with Tensor
print("--- Testing Batched Tensor @ ---")
# t_batch1 is [2, 2, 3]
let t_batch1 = Tensor([2, 2, 3])
t_batch1[(0, 0, 0)] = 1.0
t_batch1[(0, 0, 1)] = 2.0
t_batch1[(0, 0, 2)] = 3.0
t_batch1[(0, 1, 0)] = 4.0
t_batch1[(0, 1, 1)] = 5.0
t_batch1[(0, 1, 2)] = 6.0

t_batch1[(1, 0, 0)] = 1.0
t_batch1[(1, 0, 1)] = 2.0
t_batch1[(1, 0, 2)] = 3.0
t_batch1[(1, 1, 0)] = 4.0
t_batch1[(1, 1, 1)] = 5.0
t_batch1[(1, 1, 2)] = 6.0

# t_batch2 is [2, 3, 2]
let t_batch2 = Tensor([2, 3, 2])
t_batch2[(0, 0, 0)] = 7.0
t_batch2[(0, 0, 1)] = 8.0
t_batch2[(0, 1, 0)] = 9.0
t_batch2[(0, 1, 1)] = 10.0
t_batch2[(0, 2, 0)] = 11.0
t_batch2[(0, 2, 1)] = 12.0

t_batch2[(1, 0, 0)] = 7.0
t_batch2[(1, 0, 1)] = 8.0
t_batch2[(1, 1, 0)] = 9.0
t_batch2[(1, 1, 1)] = 10.0
t_batch2[(1, 2, 0)] = 11.0
t_batch2[(1, 2, 1)] = 12.0

let tb_res = t_batch1 @ t_batch2
print("tb_res[0, 0, 0] =")  # 58.0
print(tb_res[(0, 0, 0)])
print("tb_res[1, 1, 1] =")  # 154.0
print(tb_res[(1, 1, 1)])

# 5. Test shape manipulation methods
print("--- Testing Shape Manipulation Methods ---")
let t_orig = Tensor([2, 3])
t_orig[(0, 0)] = 1.0
t_orig[(0, 1)] = 2.0
t_orig[(0, 2)] = 3.0
t_orig[(1, 0)] = 4.0
t_orig[(1, 1)] = 5.0
t_orig[(1, 2)] = 6.0

# reshape
let t_reshaped = t_orig.reshape([6])
print("reshaped[5] =")
print(t_reshaped[5])  # should be 6.0

# transpose
let t_transposed = t_orig.transpose(0, 1) # [3, 2]
print("transposed[2, 1] =")
print(t_transposed[(2, 1)])  # should be t_orig[1, 2] = 6.0

# squeeze
let t_sq_orig = Tensor([1, 2, 1, 3])
t_sq_orig[(0, 0, 0, 0)] = 1.0
t_sq_orig[(0, 0, 0, 1)] = 2.0
t_sq_orig[(0, 0, 0, 2)] = 3.0
t_sq_orig[(0, 1, 0, 0)] = 4.0
t_sq_orig[(0, 1, 0, 1)] = 5.0
t_sq_orig[(0, 1, 0, 2)] = 6.0

let t_squeezed = t_sq_orig.squeeze(2) # should be [1, 2, 3]
print("squeezed[0, 1, 2] =")
print(t_squeezed[(0, 1, 2)])  # should be 6.0

# 6. Test direct subscript loops
print("--- Testing Loop Indexing ---")
let t_loop = Tensor([10, 10])
let i = 0
while i < 10:
    let j = 0
    while j < 10:
        t_loop[(i, j)] = (i * 10 + j) * 1.0
        j = j + 1
    i = i + 1

print("t_loop[9, 9] =")
print(t_loop[(9, 9)])  # should be 99.0

print("Finished matmul and shape tests!")
