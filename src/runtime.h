#ifndef MPS_RUNTIME_H
#define MPS_RUNTIME_H

#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdarg.h>
#include <math.h>
#include <string.h>
#include <setjmp.h>
#include <stdint.h>
#include <time.h>

#ifdef MPS_USE_PYTHON
#include <Python.h>
#else
// Mock PyObject for native collections when Python is not imported
typedef enum {
    OBJ_INT,
    OBJ_FLOAT,
    OBJ_STRING,
    OBJ_BOOL,
    OBJ_LIST,
    OBJ_TUPLE,
    OBJ_DICT,
    OBJ_ITER,
    OBJ_SLICE,
    OBJ_NULL
} ObjType;

typedef struct PyObject PyObject;

typedef struct {
    PyObject** items;
    int size;
    int capacity;
} MPS_ListImpl;

typedef struct {
    PyObject* key;
    PyObject* val;
} MPS_DictPair;

typedef struct {
    MPS_DictPair* pairs;
    int size;
    int capacity;
} MPS_DictImpl;

typedef struct {
    PyObject* obj;
    int index;
} MPS_IterImpl;

typedef struct {
    PyObject* start;
    PyObject* stop;
    PyObject* step;
} MPS_SliceImpl;

struct PyObject {
    ObjType type;
    int ref_count;
    union {
        int int_val;
        double float_val;
        const char* string_val;
        bool bool_val;
        MPS_ListImpl* list_val;
        MPS_DictImpl* dict_val;
        MPS_IterImpl* iter_val;
        MPS_SliceImpl* slice_val;
    } value;
};
extern PyObject* _mps_py_none_ref;
#define Py_None _mps_py_none_ref

static inline const char* mps_str_get_char(const char* s, int index);
static inline const char* mps_str_upper(const char* s);
static inline const char* mps_str_lower(const char* s);
static inline const char* mps_str_trim(const char* s);
static inline bool mps_str_starts_with(const char* s, const char* prefix);
static inline bool mps_str_ends_with(const char* s, const char* suffix);
static inline bool mps_str_contains(const char* s, const char* sub);
static inline const char* mps_str_replace(const char* s, const char* old_str, const char* new_str);
static inline PyObject* mps_str_split(const char* s, const char* sep);
static inline const char* mps_str_join(const char* connector, PyObject* list);
static inline PyObject* PySlice_New(PyObject* start, PyObject* stop, PyObject* step);
static inline PyObject* PySequence_GetItem(PyObject* o, int i);
#endif
static inline const char* mps_str_slice(const char* s, int start, int end);

#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#include <unistd.h>
#endif

/* --- Try/Catch Error Handling via setjmp/longjmp --- */
#ifdef _MSC_VER
#define THREAD_LOCAL __declspec(thread)
#else
#define THREAD_LOCAL _Thread_local
#endif

typedef struct {
    const char* message;
    int code;
} MPS_Error;

/* --- Call Stack for Stack Traces --- */
#define MPS_MAX_STACK_DEPTH 128

typedef struct {
    const char* func_name;
    const char* file_name;
    int line_number;
} MPS_StackFrame;

extern THREAD_LOCAL jmp_buf* mps_err_buf;
extern THREAD_LOCAL MPS_Error mps_last_error;
extern THREAD_LOCAL MPS_StackFrame mps_call_stack[MPS_MAX_STACK_DEPTH];
extern THREAD_LOCAL int mps_stack_depth;

// Define storage for thread local error context
#ifdef MPS_RUNTIME_IMPL
THREAD_LOCAL jmp_buf* mps_err_buf = NULL;
THREAD_LOCAL MPS_Error mps_last_error = { "", 0 };
THREAD_LOCAL MPS_StackFrame mps_call_stack[MPS_MAX_STACK_DEPTH];
THREAD_LOCAL int mps_stack_depth = 0;
#ifndef MPS_USE_PYTHON
PyObject _mps_py_none_val = { OBJ_NULL, 1, {0} };
PyObject* _mps_py_none_ref = &_mps_py_none_val;
#endif
#endif

static inline void mps_push_frame(const char* func, const char* file, int line) {
    if (mps_stack_depth < MPS_MAX_STACK_DEPTH) {
        mps_call_stack[mps_stack_depth].func_name = func;
        mps_call_stack[mps_stack_depth].file_name = file;
        mps_call_stack[mps_stack_depth].line_number = line;
        mps_stack_depth++;
    }
}

static inline void mps_pop_frame(void) {
    if (mps_stack_depth > 0) {
        mps_stack_depth--;
    }
}

static inline void mps_print_stack_trace(void) {
    if (mps_stack_depth > 0) {
        fprintf(stderr, "\nStack Trace (most recent call last):\n");
        for (int i = mps_stack_depth - 1; i >= 0; i--) {
            fprintf(stderr, "  [%d] %s", mps_stack_depth - i, mps_call_stack[i].func_name);
            if (mps_call_stack[i].file_name && mps_call_stack[i].file_name[0]) {
                fprintf(stderr, " (%s", mps_call_stack[i].file_name);
                if (mps_call_stack[i].line_number > 0) {
                    fprintf(stderr, ":%d", mps_call_stack[i].line_number);
                }
                fprintf(stderr, ")");
            }
            fprintf(stderr, "\n");
        }
    }
}

static inline void mps_raise(const char* message) {
    mps_last_error.message = message;
    mps_last_error.code = 1;
    if (mps_err_buf != NULL) {
        longjmp(*mps_err_buf, 1);
    } else {
        fprintf(stderr, "Unhandled Exception: %s\n", message);
        mps_print_stack_trace();
        exit(1);
    }
}

/* --- Native Fixed-Size Thread Pool for Async/Await --- */
#define MPS_MAX_POOL_THREADS 64

typedef struct MPS_Task {
    void (*fn)(void*);
    void* arg;
    bool completed;
#ifdef _WIN32
    CONDITION_VARIABLE cv;
    CRITICAL_SECTION cs;
#else
    pthread_cond_t cv;
    pthread_mutex_t cs;
#endif
    struct MPS_Task* next;
} MPS_Task;

#ifdef MPS_RUNTIME_IMPL
static MPS_Task* queue_head = NULL;
static MPS_Task* queue_tail = NULL;
static bool pool_shutdown = false;
static int pool_thread_count = 4;
static bool pool_initialized = false;

static inline int mps_pool_requested_thread_count(void) {
    const char* env = getenv("MPS_POOL_THREADS");
    if (env == NULL || env[0] == '\0') {
        env = getenv("MPS_THREADS");
    }
    if (env == NULL || env[0] == '\0') {
        return pool_thread_count;
    }
    int parsed = atoi(env);
    if (parsed < 1) {
        return 1;
    }
    if (parsed > MPS_MAX_POOL_THREADS) {
        return MPS_MAX_POOL_THREADS;
    }
    return parsed;
}

#ifdef _WIN32
static CRITICAL_SECTION pool_cs;
static CONDITION_VARIABLE pool_cv;
static HANDLE pool_threads[MPS_MAX_POOL_THREADS];

static DWORD WINAPI worker_thread(LPVOID lpParam) {
    (void)lpParam;
    while (1) {
        MPS_Task* task = NULL;
        EnterCriticalSection(&pool_cs);
        while (queue_head == NULL && !pool_shutdown) {
            SleepConditionVariableCS(&pool_cv, &pool_cs, INFINITE);
        }
        if (pool_shutdown) {
            LeaveCriticalSection(&pool_cs);
            break;
        }
        task = queue_head;
        if (queue_head != NULL) {
            queue_head = queue_head->next;
            if (queue_head == NULL) {
                queue_tail = NULL;
            }
        }
        LeaveCriticalSection(&pool_cs);

        if (task != NULL) {
            task->fn(task->arg);

            EnterCriticalSection(&task->cs);
            task->completed = true;
            LeaveCriticalSection(&task->cs);
            WakeAllConditionVariable(&task->cv);
        }
    }
    return 0;
}
#else
static pthread_mutex_t pool_cs = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t pool_cv = PTHREAD_COND_INITIALIZER;
static pthread_t pool_threads[MPS_MAX_POOL_THREADS];

static void* worker_thread(void* lpParam) {
    (void)lpParam;
    while (1) {
        MPS_Task* task = NULL;
        pthread_mutex_lock(&pool_cs);
        while (queue_head == NULL && !pool_shutdown) {
            pthread_cond_wait(&pool_cv, &pool_cs);
        }
        if (pool_shutdown) {
            pthread_mutex_unlock(&pool_cs);
            break;
        }
        task = queue_head;
        if (queue_head != NULL) {
            queue_head = queue_head->next;
            if (queue_head == NULL) {
                queue_tail = NULL;
            }
        }
        pthread_mutex_unlock(&pool_cs);

        if (task != NULL) {
            task->fn(task->arg);

            pthread_mutex_lock(&task->cs);
            task->completed = true;
            pthread_mutex_unlock(&task->cs);
            pthread_cond_broadcast(&task->cv);
        }
    }
    return NULL;
}
#endif

static inline void mps_pool_init() {
#ifdef MPS_RUNTIME_IMPL
    if (pool_initialized) return;
    pool_thread_count = mps_pool_requested_thread_count();
#ifdef _WIN32
    InitializeCriticalSection(&pool_cs);
    InitializeConditionVariable(&pool_cv);
    for (int i = 0; i < pool_thread_count; i++) {
        pool_threads[i] = CreateThread(NULL, 0, worker_thread, NULL, 0, NULL);
    }
#else
    pthread_mutex_init(&pool_cs, NULL);
    pthread_cond_init(&pool_cv, NULL);
    for (int i = 0; i < pool_thread_count; i++) {
        pthread_create(&pool_threads[i], NULL, worker_thread, NULL);
    }
#endif
    pool_initialized = true;
#endif
}

static inline void mps_pool_submit(MPS_Task* task) {
#ifdef MPS_RUNTIME_IMPL
    mps_pool_init();
    task->completed = false;
#ifdef _WIN32
    InitializeConditionVariable(&task->cv);
    InitializeCriticalSection(&task->cs);
#else
    pthread_cond_init(&task->cv, NULL);
    pthread_mutex_init(&task->cs, NULL);
#endif
    task->next = NULL;

#ifdef _WIN32
    EnterCriticalSection(&pool_cs);
    if (queue_tail == NULL) {
        queue_head = task;
        queue_tail = task;
    } else {
        queue_tail->next = task;
        queue_tail = task;
    }
    LeaveCriticalSection(&pool_cs);
    WakeConditionVariable(&pool_cv);
#else
    pthread_mutex_lock(&pool_cs);
    if (queue_tail == NULL) {
        queue_head = task;
        queue_tail = task;
    } else {
        queue_tail->next = task;
        queue_tail = task;
    }
    pthread_mutex_unlock(&pool_cs);
    pthread_cond_signal(&pool_cv);
#endif
#else
    (void)task;
#endif
}

static inline void mps_task_await(MPS_Task* task) {
#ifdef _WIN32
    EnterCriticalSection(&task->cs);
    while (!task->completed) {
        SleepConditionVariableCS(&task->cv, &task->cs, INFINITE);
    }
    LeaveCriticalSection(&task->cs);
#else
    pthread_mutex_lock(&task->cs);
    while (!task->completed) {
        pthread_cond_wait(&task->cv, &task->cs);
    }
    pthread_mutex_unlock(&task->cs);
#endif
}

#endif /* MPS_RUNTIME_IMPL */

/* --- Basic Types and Collections FFI / Native Fallback --- */
#ifdef MPS_USE_PYTHON
static inline PyObject* _to_py_string(const char* s) {
    return PyUnicode_FromString(s);
}

static inline PyObject* _to_py_int(int i) {
    return PyLong_FromLong(i);
}

static inline PyObject* _to_py_double(double d) {
    return PyFloat_FromDouble(d);
}

static inline PyObject* _to_py_bool(bool b) {
    return PyBool_FromLong(b);
}

static inline PyObject* _to_py_pyobject(PyObject* obj) {
    Py_XINCREF(obj);
    return obj;
}

#define to_py(X) _Generic((X), \
    const char*: _to_py_string, \
    char*: _to_py_string, \
    int: _to_py_int, \
    double: _to_py_double, \
    float: _to_py_double, \
    bool: _to_py_bool, \
    PyObject*: _to_py_pyobject \
)(X)

static inline PyObject* mps_list_new(int count, ...) {
    PyObject* list = PyList_New(count);
    va_list ap;
    va_start(ap, count);
    for (int i = 0; i < count; i++) {
        PyObject* item = va_arg(ap, PyObject*);
        PyList_SetItem(list, i, item); // steals item reference
    }
    va_end(ap);
    return list;
}

static inline PyObject* mps_tuple_new(int count, ...) {
    PyObject* tuple = PyTuple_New(count);
    va_list ap;
    va_start(ap, count);
    for (int i = 0; i < count; i++) {
        PyObject* item = va_arg(ap, PyObject*);
        PyTuple_SetItem(tuple, i, item); // steals item reference
    }
    va_end(ap);
    return tuple;
}

static inline PyObject* mps_dict_new(int pair_count, ...) {
    PyObject* dict = PyDict_New();
    va_list ap;
    va_start(ap, pair_count);
    for (int i = 0; i < pair_count; i++) {
        PyObject* key = va_arg(ap, PyObject*);
        PyObject* val = va_arg(ap, PyObject*);
        PyDict_SetItem(dict, key, val);
        Py_DECREF(key);
        Py_DECREF(val);
    }
    va_end(ap);
    return dict;
}
#else /* Native Fallback when MPS_USE_PYTHON is not defined */
static inline PyObject* mps_obj_alloc(ObjType type) {
    PyObject* obj = (PyObject*)malloc(sizeof(PyObject));
    obj->type = type;
    obj->ref_count = 1;
    return obj;
}

static inline void Py_XINCREF(PyObject* obj) {
    if (obj != NULL) {
        obj->ref_count++;
    }
}

static inline void Py_XDECREF(PyObject* obj) {
    if (obj == NULL) return;
    obj->ref_count--;
    if (obj->ref_count <= 0) {
        if (obj->type == OBJ_LIST || obj->type == OBJ_TUPLE) {
            if (obj->value.list_val != NULL) {
                for (int i = 0; i < obj->value.list_val->size; i++) {
                    Py_XDECREF(obj->value.list_val->items[i]);
                }
                free(obj->value.list_val->items);
                free(obj->value.list_val);
            }
        } else if (obj->type == OBJ_DICT) {
            if (obj->value.dict_val != NULL) {
                for (int i = 0; i < obj->value.dict_val->size; i++) {
                    Py_XDECREF(obj->value.dict_val->pairs[i].key);
                    Py_XDECREF(obj->value.dict_val->pairs[i].val);
                }
                free(obj->value.dict_val->pairs);
                free(obj->value.dict_val);
            }
        } else if (obj->type == OBJ_ITER) {
            if (obj->value.iter_val != NULL) {
                Py_XDECREF(obj->value.iter_val->obj);
                free(obj->value.iter_val);
            }
        }
        free(obj);
    }
}

static inline void Py_DECREF(PyObject* obj) {
    Py_XDECREF(obj);
}

static inline PyObject* _to_py_string(const char* s) {
    PyObject* obj = mps_obj_alloc(OBJ_STRING);
    obj->value.string_val = s;
    return obj;
}

static inline PyObject* _to_py_int(int i) {
    PyObject* obj = mps_obj_alloc(OBJ_INT);
    obj->value.int_val = i;
    return obj;
}

static inline PyObject* _to_py_double(double d) {
    PyObject* obj = mps_obj_alloc(OBJ_FLOAT);
    obj->value.float_val = d;
    return obj;
}

static inline PyObject* _to_py_bool(bool b) {
    PyObject* obj = mps_obj_alloc(OBJ_BOOL);
    obj->value.bool_val = b;
    return obj;
}

static inline PyObject* _to_py_pyobject(PyObject* obj) {
    Py_XINCREF(obj);
    return obj;
}

#define to_py(X) _Generic((X), \
    const char*: _to_py_string, \
    char*: _to_py_string, \
    int: _to_py_int, \
    double: _to_py_double, \
    float: _to_py_double, \
    bool: _to_py_bool, \
    PyObject*: _to_py_pyobject \
)(X)

static inline PyObject* PyList_New(int size) {
    PyObject* obj = mps_obj_alloc(OBJ_LIST);
    obj->value.list_val = (MPS_ListImpl*)malloc(sizeof(MPS_ListImpl));
    obj->value.list_val->items = (PyObject**)malloc(sizeof(PyObject*) * (size > 0 ? size : 4));
    obj->value.list_val->size = size;
    obj->value.list_val->capacity = size > 0 ? size : 4;
    for (int i = 0; i < size; i++) obj->value.list_val->items[i] = NULL;
    return obj;
}

static inline int PyList_SetItem(PyObject* list, int index, PyObject* item) {
    if (list != NULL && list->type == OBJ_LIST && index >= 0 && index < list->value.list_val->size) {
        list->value.list_val->items[index] = item; // steals reference
        return 0;
    }
    return -1;
}

static inline int PyList_Append(PyObject* list, PyObject* item) {
    if (list == NULL || list->type != OBJ_LIST) return -1;
    MPS_ListImpl* impl = list->value.list_val;
    if (impl->size >= impl->capacity) {
        impl->capacity = impl->capacity * 2 + 1;
        impl->items = (PyObject**)realloc(impl->items, sizeof(PyObject*) * impl->capacity);
    }
    Py_XINCREF(item);
    impl->items[impl->size] = item;
    impl->size++;
    return 0;
}

static inline PyObject* mps_list_new(int count, ...) {
    PyObject* list = PyList_New(count);
    va_list ap;
    va_start(ap, count);
    for (int i = 0; i < count; i++) {
        PyObject* item = va_arg(ap, PyObject*);
        PyList_SetItem(list, i, item);
    }
    va_end(ap);
    return list;
}

static inline PyObject* PyTuple_New(int size) {
    PyObject* obj = mps_obj_alloc(OBJ_TUPLE);
    obj->value.list_val = (MPS_ListImpl*)malloc(sizeof(MPS_ListImpl));
    obj->value.list_val->items = (PyObject**)malloc(sizeof(PyObject*) * (size > 0 ? size : 4));
    obj->value.list_val->size = size;
    obj->value.list_val->capacity = size > 0 ? size : 4;
    for (int i = 0; i < size; i++) obj->value.list_val->items[i] = NULL;
    return obj;
}

static inline int PyTuple_SetItem(PyObject* tuple, int index, PyObject* item) {
    if (tuple != NULL && tuple->type == OBJ_TUPLE && index >= 0 && index < tuple->value.list_val->size) {
        tuple->value.list_val->items[index] = item; // steals reference
        return 0;
    }
    return -1;
}

static inline PyObject* mps_tuple_new(int count, ...) {
    PyObject* tuple = PyTuple_New(count);
    va_list ap;
    va_start(ap, count);
    for (int i = 0; i < count; i++) {
        PyObject* item = va_arg(ap, PyObject*);
        PyTuple_SetItem(tuple, i, item);
    }
    va_end(ap);
    return tuple;
}

static inline PyObject* PyDict_New() {
    PyObject* obj = mps_obj_alloc(OBJ_DICT);
    obj->value.dict_val = (MPS_DictImpl*)malloc(sizeof(MPS_DictImpl));
    obj->value.dict_val->pairs = (MPS_DictPair*)malloc(sizeof(MPS_DictPair) * 4);
    obj->value.dict_val->size = 0;
    obj->value.dict_val->capacity = 4;
    return obj;
}

static inline int PyDict_SetItem(PyObject* dict, PyObject* key, PyObject* val) {
    if (dict != NULL && dict->type == OBJ_DICT) {
        MPS_DictImpl* impl = dict->value.dict_val;
        for (int i = 0; i < impl->size; i++) {
            bool match = false;
            PyObject* k = impl->pairs[i].key;
            if (k->type == key->type) {
                if (k->type == OBJ_INT) match = k->value.int_val == key->value.int_val;
                else if (k->type == OBJ_STRING) match = strcmp(k->value.string_val, key->value.string_val) == 0;
                else if (k->type == OBJ_FLOAT) match = k->value.float_val == key->value.float_val;
                else if (k->type == OBJ_BOOL) match = k->value.bool_val == key->value.bool_val;
            }
            if (match) {
                Py_XDECREF(impl->pairs[i].val);
                Py_XINCREF(val);
                impl->pairs[i].val = val;
                return 0;
            }
        }
        if (impl->size >= impl->capacity) {
            impl->capacity *= 2;
            impl->pairs = (MPS_DictPair*)realloc(impl->pairs, sizeof(MPS_DictPair) * impl->capacity);
        }
        Py_XINCREF(key);
        Py_XINCREF(val);
        impl->pairs[impl->size].key = key;
        impl->pairs[impl->size].val = val;
        impl->size++;
        return 0;
    }
    return -1;
}

static inline PyObject* mps_dict_new(int pair_count, ...) {
    PyObject* dict = PyDict_New();
    va_list ap;
    va_start(ap, pair_count);
    for (int i = 0; i < pair_count; i++) {
        PyObject* key = va_arg(ap, PyObject*);
        PyObject* val = va_arg(ap, PyObject*);
        PyDict_SetItem(dict, key, val);
        Py_DECREF(key);
        Py_DECREF(val);
    }
    va_end(ap);
    return dict;
}

#ifndef MPS_USE_PYTHON
static inline PyObject* PySlice_New(PyObject* start, PyObject* stop, PyObject* step) {
    PyObject* obj = mps_obj_alloc(OBJ_SLICE);
    obj->value.slice_val = (MPS_SliceImpl*)malloc(sizeof(MPS_SliceImpl));
    Py_XINCREF(start);
    Py_XINCREF(stop);
    Py_XINCREF(step);
    obj->value.slice_val->start = start;
    obj->value.slice_val->stop = stop;
    obj->value.slice_val->step = step;
    return obj;
}

static inline PyObject* PySequence_GetItem(PyObject* o, int i) {
    if (o == NULL) return NULL;
    if (o->type == OBJ_LIST || o->type == OBJ_TUPLE) {
        if (i >= 0 && i < o->value.list_val->size) {
            PyObject* item = o->value.list_val->items[i];
            Py_XINCREF(item);
            return item;
        }
    } else if (o->type == OBJ_STRING) {
        const char* char_str = mps_str_get_char(o->value.string_val, i);
        return _to_py_string(char_str);
    }
    return NULL;
}
#endif

static inline const char* mps_str_slice(const char* s, int start, int end) {
    if (s == NULL) return "";
    int len = (int)strlen(s);
    int real_start = (start == -1) ? 0 : (start < 0 ? len + start : start);
    int real_end = (end == -1) ? len : (end < 0 ? len + end : end);
    
    if (real_start < 0) real_start = 0;
    if (real_start > len) real_start = len;
    if (real_end < 0) real_end = 0;
    if (real_end > len) real_end = len;
    if (real_end < real_start) real_end = real_start;
    
    int slice_len = real_end - real_start;
    char* res = (char*)malloc(slice_len + 1);
    memcpy(res, s + real_start, slice_len);
    res[slice_len] = '\0';
    return res;
}

static inline PyObject* PyObject_GetItem(PyObject* obj, PyObject* key) {
    if (obj == NULL || key == NULL) return NULL;
    if (key->type == OBJ_SLICE) {
        if (obj->type == OBJ_LIST || obj->type == OBJ_TUPLE) {
            MPS_SliceImpl* slice = key->value.slice_val;
            int size = obj->value.list_val->size;
            
            int start = 0;
            if (slice->start != NULL && slice->start != Py_None && slice->start->type == OBJ_INT) {
                start = slice->start->value.int_val;
                if (start < 0) start += size;
            }
            if (start < 0) start = 0;
            if (start > size) start = size;
            
            int stop = size;
            if (slice->stop != NULL && slice->stop != Py_None && slice->stop->type == OBJ_INT) {
                stop = slice->stop->value.int_val;
                if (stop < 0) stop += size;
            }
            if (stop < 0) stop = 0;
            if (stop > size) stop = size;
            
            int step = 1;
            if (slice->step != NULL && slice->step != Py_None && slice->step->type == OBJ_INT) {
                step = slice->step->value.int_val;
            }
            if (step == 0) step = 1;
            
            int slice_size = 0;
            if (step > 0) {
                for (int i = start; i < stop; i += step) slice_size++;
            } else {
                for (int i = start; i > stop; i += step) slice_size++;
            }
            
            PyObject* result = (obj->type == OBJ_LIST) ? PyList_New(slice_size) : PyTuple_New(slice_size);
            int idx = 0;
            if (step > 0) {
                for (int i = start; i < stop; i += step) {
                    PyObject* item = obj->value.list_val->items[i];
                    Py_XINCREF(item);
                    if (obj->type == OBJ_LIST) {
                        PyList_SetItem(result, idx++, item);
                    } else {
                        PyTuple_SetItem(result, idx++, item);
                    }
                }
            } else {
                for (int i = start; i > stop; i += step) {
                    PyObject* item = obj->value.list_val->items[i];
                    Py_XINCREF(item);
                    if (obj->type == OBJ_LIST) {
                        PyList_SetItem(result, idx++, item);
                    } else {
                        PyTuple_SetItem(result, idx++, item);
                    }
                }
            }
            return result;
        } else if (obj->type == OBJ_STRING) {
            MPS_SliceImpl* slice = key->value.slice_val;
            int len = (int)strlen(obj->value.string_val);
            int start = -1;
            if (slice->start != NULL && slice->start != Py_None && slice->start->type == OBJ_INT) {
                start = slice->start->value.int_val;
            }
            int stop = -1;
            if (slice->stop != NULL && slice->stop != Py_None && slice->stop->type == OBJ_INT) {
                stop = slice->stop->value.int_val;
            }
            const char* sliced_str = mps_str_slice(obj->value.string_val, start, stop);
            PyObject* res_obj = _to_py_string(sliced_str);
            free((void*)sliced_str);
            return res_obj;
        }
    }
    if (obj->type == OBJ_LIST || obj->type == OBJ_TUPLE) {
        int idx = key->type == OBJ_INT ? key->value.int_val : 0;
        if (idx >= 0 && idx < obj->value.list_val->size) {
            PyObject* item = obj->value.list_val->items[idx];
            Py_XINCREF(item);
            return item;
        }
    } else if (obj->type == OBJ_DICT) {
        MPS_DictImpl* impl = obj->value.dict_val;
        for (int i = 0; i < impl->size; i++) {
            bool match = false;
            PyObject* k = impl->pairs[i].key;
            if (k->type == key->type) {
                if (k->type == OBJ_INT) match = k->value.int_val == key->value.int_val;
                else if (k->type == OBJ_STRING) match = strcmp(k->value.string_val, key->value.string_val) == 0;
                else if (k->type == OBJ_FLOAT) match = k->value.float_val == key->value.float_val;
                else if (k->type == OBJ_BOOL) match = k->value.bool_val == key->value.bool_val;
            }
            if (match) {
                PyObject* val = impl->pairs[i].val;
                Py_XINCREF(val);
                return val;
            }
        }
    } else if (obj->type == OBJ_STRING) {
        int idx = key->type == OBJ_INT ? key->value.int_val : 0;
        const char* char_str = mps_str_get_char(obj->value.string_val, idx);
        return _to_py_string(char_str);
    }
    return NULL;
}

static inline int PyObject_SetItem(PyObject* obj, PyObject* key, PyObject* val) {
    if (obj == NULL || key == NULL || val == NULL) return -1;
    if (obj->type == OBJ_LIST) {
        int idx = key->type == OBJ_INT ? key->value.int_val : 0;
        if (idx >= 0 && idx < obj->value.list_val->size) {
            Py_XDECREF(obj->value.list_val->items[idx]);
            Py_XINCREF(val);
            obj->value.list_val->items[idx] = val;
            return 0;
        }
    } else if (obj->type == OBJ_TUPLE) {
        int idx = key->type == OBJ_INT ? key->value.int_val : 0;
        if (idx >= 0 && idx < obj->value.list_val->size) {
            Py_XDECREF(obj->value.list_val->items[idx]);
            Py_XINCREF(val);
            obj->value.list_val->items[idx] = val;
            return 0;
        }
    } else if (obj->type == OBJ_DICT) {
        return PyDict_SetItem(obj, key, val);
    }
    return -1;
}

static inline int PyObject_Size(PyObject* obj) {
    if (obj == NULL) return 0;
    if (obj->type == OBJ_LIST || obj->type == OBJ_TUPLE) return obj->value.list_val->size;
    if (obj->type == OBJ_DICT) return obj->value.dict_val->size;
    if (obj->type == OBJ_STRING) return (int)strlen(obj->value.string_val);
    return 0;
}
#define PyObject_Length PyObject_Size

static inline PyObject* py_call(PyObject* obj, const char* name, int argc, ...) {
    PyObject* ret = NULL;
    PyObject* args[16];
    int max_args = argc < 16 ? argc : 16;
    va_list ap;
    va_start(ap, argc);
    for (int i = 0; i < max_args; i++) {
        args[i] = va_arg(ap, PyObject*);
    }
    va_end(ap);

    if (obj != NULL) {
        if (obj->type == OBJ_LIST) {
            if (strcmp(name, "append") == 0 && max_args == 1) {
                PyObject* val = args[0];
                MPS_ListImpl* impl = obj->value.list_val;
                if (impl->size >= impl->capacity) {
                    impl->capacity = impl->capacity * 2 + 1;
                    impl->items = (PyObject**)realloc(impl->items, sizeof(PyObject*) * impl->capacity);
                }
                Py_XINCREF(val);
                impl->items[impl->size] = val;
                impl->size++;
            }
            else if (strcmp(name, "pop") == 0) {
                MPS_ListImpl* impl = obj->value.list_val;
                if (impl->size > 0) {
                    int idx = impl->size - 1;
                    if (max_args == 1 && args[0] != NULL && args[0]->type == OBJ_INT) {
                        idx = args[0]->value.int_val;
                    }
                    if (idx >= 0 && idx < impl->size) {
                        ret = impl->items[idx];
                        for (int i = idx; i < impl->size - 1; i++) {
                            impl->items[i] = impl->items[i + 1];
                        }
                        impl->size--;
                    }
                }
            }
            else if (strcmp(name, "remove") == 0 && max_args == 1) {
                PyObject* target = args[0];
                MPS_ListImpl* impl = obj->value.list_val;
                int found_idx = -1;
                for (int i = 0; i < impl->size; i++) {
                    PyObject* item = impl->items[i];
                    bool match = false;
                    if (item->type == target->type) {
                        if (item->type == OBJ_INT) match = item->value.int_val == target->value.int_val;
                        else if (item->type == OBJ_FLOAT) match = item->value.float_val == target->value.float_val;
                        else if (item->type == OBJ_STRING) match = strcmp(item->value.string_val, target->value.string_val) == 0;
                        else if (item->type == OBJ_BOOL) match = item->value.bool_val == target->value.bool_val;
                    }
                    if (match) {
                        found_idx = i;
                        break;
                    }
                }
                if (found_idx != -1) {
                    Py_XDECREF(impl->items[found_idx]);
                    for (int i = found_idx; i < impl->size - 1; i++) {
                        impl->items[i] = impl->items[i + 1];
                    }
                    impl->size--;
                }
            }
            else if (strcmp(name, "clear") == 0) {
                MPS_ListImpl* impl = obj->value.list_val;
                for (int i = 0; i < impl->size; i++) {
                    Py_XDECREF(impl->items[i]);
                }
                impl->size = 0;
            }
            else if (strcmp(name, "length") == 0) {
                ret = _to_py_int(obj->value.list_val->size);
            }
        }
        else if (obj->type == OBJ_DICT) {
            if (strcmp(name, "clear") == 0) {
                MPS_DictImpl* impl = obj->value.dict_val;
                for (int i = 0; i < impl->size; i++) {
                    Py_XDECREF(impl->pairs[i].key);
                    Py_XDECREF(impl->pairs[i].val);
                }
                impl->size = 0;
            }
            else if (strcmp(name, "keys") == 0) {
                MPS_DictImpl* impl = obj->value.dict_val;
                PyObject* list = PyList_New(impl->size);
                for (int i = 0; i < impl->size; i++) {
                    PyObject* k = impl->pairs[i].key;
                    Py_XINCREF(k);
                    PyList_SetItem(list, i, k);
                }
                ret = list;
            }
            else if (strcmp(name, "values") == 0) {
                MPS_DictImpl* impl = obj->value.dict_val;
                PyObject* list = PyList_New(impl->size);
                for (int i = 0; i < impl->size; i++) {
                    PyObject* v = impl->pairs[i].val;
                    Py_XINCREF(v);
                    PyList_SetItem(list, i, v);
                }
                ret = list;
            }
            else if (strcmp(name, "get") == 0 && max_args >= 1) {
                PyObject* key = args[0];
                PyObject* def_val = max_args >= 2 ? args[1] : mps_obj_alloc(OBJ_NULL);
                MPS_DictImpl* impl = obj->value.dict_val;
                bool found = false;
                for (int i = 0; i < impl->size; i++) {
                    bool match = false;
                    PyObject* k = impl->pairs[i].key;
                    if (k->type == key->type) {
                        if (k->type == OBJ_INT) match = k->value.int_val == key->value.int_val;
                        else if (k->type == OBJ_STRING) match = strcmp(k->value.string_val, key->value.string_val) == 0;
                        else if (k->type == OBJ_FLOAT) match = k->value.float_val == key->value.float_val;
                        else if (k->type == OBJ_BOOL) match = k->value.bool_val == key->value.bool_val;
                    }
                    if (match) {
                        ret = impl->pairs[i].val;
                        Py_XINCREF(ret);
                        found = true;
                        break;
                    }
                }
                if (!found) {
                    ret = def_val;
                    Py_XINCREF(ret);
                }
            }
            else if (strcmp(name, "contains") == 0 && max_args == 1) {
                PyObject* key = args[0];
                MPS_DictImpl* impl = obj->value.dict_val;
                bool found = false;
                for (int i = 0; i < impl->size; i++) {
                    bool match = false;
                    PyObject* k = impl->pairs[i].key;
                    if (k->type == key->type) {
                        if (k->type == OBJ_INT) match = k->value.int_val == key->value.int_val;
                        else if (k->type == OBJ_STRING) match = strcmp(k->value.string_val, key->value.string_val) == 0;
                        else if (k->type == OBJ_FLOAT) match = k->value.float_val == key->value.float_val;
                        else if (k->type == OBJ_BOOL) match = k->value.bool_val == key->value.bool_val;
                    }
                    if (match) {
                        found = true;
                        break;
                    }
                }
                ret = _to_py_bool(found);
            }
            else if (strcmp(name, "length") == 0) {
                ret = _to_py_int(obj->value.dict_val->size);
            }
        }
        else if (obj->type == OBJ_STRING) {
            const char* s = obj->value.string_val;
            if (strcmp(name, "upper") == 0) {
                ret = _to_py_string(mps_str_upper(s));
            }
            else if (strcmp(name, "lower") == 0) {
                ret = _to_py_string(mps_str_lower(s));
            }
            else if (strcmp(name, "trim") == 0) {
                ret = _to_py_string(mps_str_trim(s));
            }
            else if (strcmp(name, "startswith") == 0 && max_args == 1 && args[0] != NULL && args[0]->type == OBJ_STRING) {
                ret = _to_py_bool(mps_str_starts_with(s, args[0]->value.string_val));
            }
            else if (strcmp(name, "endswith") == 0 && max_args == 1 && args[0] != NULL && args[0]->type == OBJ_STRING) {
                ret = _to_py_bool(mps_str_ends_with(s, args[0]->value.string_val));
            }
            else if (strcmp(name, "contains") == 0 && max_args == 1 && args[0] != NULL && args[0]->type == OBJ_STRING) {
                ret = _to_py_bool(mps_str_contains(s, args[0]->value.string_val));
            }
            else if (strcmp(name, "replace") == 0 && max_args == 2 && args[0] != NULL && args[0]->type == OBJ_STRING && args[1] != NULL && args[1]->type == OBJ_STRING) {
                ret = _to_py_string(mps_str_replace(s, args[0]->value.string_val, args[1]->value.string_val));
            }
            else if (strcmp(name, "split") == 0) {
                const char* sep = " ";
                if (max_args == 1 && args[0] != NULL && args[0]->type == OBJ_STRING) {
                    sep = args[0]->value.string_val;
                }
                ret = mps_str_split(s, sep);
            }
            else if (strcmp(name, "join") == 0 && max_args == 1) {
                ret = _to_py_string(mps_str_join(s, args[0]));
            }
        }
    }

    for (int i = 0; i < max_args; i++) {
        Py_XDECREF(args[i]);
    }
    return ret;
}


static inline PyObject* PyObject_GetAttrString(PyObject* obj, const char* name) {
    (void)name;
    Py_XINCREF(obj);
    return obj;
}

static inline int PyObject_SetAttrString(PyObject* obj, const char* name, PyObject* val) {
    (void)obj;
    (void)name;
    (void)val;
    return 0;
}

static inline bool PyObject_IsTrue(PyObject* obj) {
    if (obj == NULL) return false;
    switch (obj->type) {
        case OBJ_INT: return obj->value.int_val != 0;
        case OBJ_FLOAT: return obj->value.float_val != 0.0;
        case OBJ_STRING: return strlen(obj->value.string_val) > 0;
        case OBJ_BOOL: return obj->value.bool_val;
        case OBJ_LIST:
        case OBJ_TUPLE: return obj->value.list_val->size > 0;
        case OBJ_DICT: return obj->value.dict_val->size > 0;
        default: return false;
    }
}

static inline PyObject* PyUnicode_DecodeFSDefault(const char* s) {
    return _to_py_string(s);
}

static inline PyObject* PyImport_Import(PyObject* name) {
    (void)name;
    return mps_obj_alloc(OBJ_NULL);
}

static inline PyObject* PyObject_GetIter(PyObject* obj) {
    PyObject* iter = mps_obj_alloc(OBJ_ITER);
    iter->value.iter_val = (MPS_IterImpl*)malloc(sizeof(MPS_IterImpl));
    iter->value.iter_val->obj = obj;
    Py_XINCREF(obj);
    iter->value.iter_val->index = 0;
    return iter;
}

static inline PyObject* PyIter_Next(PyObject* iter) {
    if (iter == NULL || iter->type != OBJ_ITER) return NULL;
    MPS_IterImpl* impl = iter->value.iter_val;
    if (impl->obj == NULL) return NULL;
    if (impl->obj->type == OBJ_LIST || impl->obj->type == OBJ_TUPLE) {
        MPS_ListImpl* list = impl->obj->value.list_val;
        if (impl->index < list->size) {
            PyObject* item = list->items[impl->index];
            Py_XINCREF(item);
            impl->index++;
            return item;
        }
    } else if (impl->obj->type == OBJ_DICT) {
        MPS_DictImpl* dict = impl->obj->value.dict_val;
        if (impl->index < dict->size) {
            PyObject* item = dict->pairs[impl->index].key;
            Py_XINCREF(item);
            impl->index++;
            return item;
        }
    }
    return NULL;
}

static inline void Py_Initialize() {}
static inline void Py_Finalize() {}
#endif /* MPS_USE_PYTHON */

/* --- Native Matrix Definition and BLAS Helpers --- */
#ifdef MPS_USE_BLAS
#ifdef _MSC_VER
#include <openblas/cblas.h>
#else
#include <cblas.h>
#endif
#ifndef CblasRowMajor
#define CblasRowMajor 101
#endif
#ifndef CblasNoTrans
#define CblasNoTrans 111
#endif
#endif

typedef struct {
    double* data;
    int rows;
    int cols;
} MPSMatrix;

typedef struct {
    float* data;
    int rows;
    int cols;
} MPSMatrix32;

static inline MPSMatrix* matrix_new(int rows, int cols) {
    MPSMatrix* m = (MPSMatrix*)malloc(sizeof(MPSMatrix));
    m->rows = rows;
    m->cols = cols;
    m->data = (double*)malloc(rows * cols * sizeof(double));
    return m;
}

static inline void matrix_free(MPSMatrix* m) {
    if (m != NULL) {
        if (m->data != NULL) {
            free(m->data);
        }
        free(m);
    }
}

static inline double matrix_get(MPSMatrix* m, int r, int c) {
    if (m == NULL || r < 0 || r >= m->rows || c < 0 || c >= m->cols) {
        fprintf(stderr, "Index Error: Matrix subscript out of bounds.\n");
        exit(1);
    }
    return m->data[r * m->cols + c];
}

static inline void matrix_set(MPSMatrix* m, int r, int c, double val) {
    if (m == NULL || r < 0 || r >= m->rows || c < 0 || c >= m->cols) {
        fprintf(stderr, "Index Error: Matrix subscript out of bounds.\n");
        exit(1);
    }
    m->data[r * m->cols + c] = val;
}

static inline MPSMatrix* matrix_mul(MPSMatrix* a, MPSMatrix* b) {
    if (a == NULL || b == NULL || a->cols != b->rows) {
        fprintf(stderr, "Error: Matrix dimensions mismatch for multiplication (%dx%d vs %dx%d).\n", 
                a ? a->rows : 0, a ? a->cols : 0, b ? b->rows : 0, b ? b->cols : 0);
        exit(1);
    }
    MPSMatrix* c = matrix_new(a->rows, b->cols);
    
#ifdef MPS_USE_BLAS
    cblas_dgemm(CblasRowMajor, CblasNoTrans, CblasNoTrans,
                a->rows, b->cols, a->cols, 1.0,
                a->data, a->cols, b->data, b->cols,
                0.0, c->data, b->cols);
#else
    int b_size = b->rows * b->cols;
    double* b_T;
    double stack_buf[1024];
    if (b_size <= 1024) {
        b_T = stack_buf;
    } else {
        static THREAD_LOCAL double* tl_buf = NULL;
        static THREAD_LOCAL int tl_cap = 0;
        if (b_size > tl_cap) {
            tl_cap = b_size * 2;
            tl_buf = (double*)realloc(tl_buf, sizeof(double) * tl_cap);
        }
        b_T = tl_buf;
    }
    
    for (int r = 0; r < b->rows; r++) {
        for (int col = 0; col < b->cols; col++) {
            b_T[col * b->rows + r] = b->data[r * b->cols + col];
        }
    }
    
    for (int i = 0; i < a->rows; i++) {
        int a_offset = i * a->cols;
        int c_offset = i * c->cols;
        for (int j = 0; j < b->cols; j++) {
            double sum = 0.0;
            int b_offset = j * b->rows;
            for (int k = 0; k < a->cols; k++) {
                sum += a->data[a_offset + k] * b_T[b_offset + k];
            }
            c->data[c_offset + j] = sum;
        }
    }
#endif
    return c;
}

static inline MPSMatrix* matrix_add(MPSMatrix* a, MPSMatrix* b) {
    if (a == NULL || b == NULL || a->rows != b->rows || a->cols != b->cols) {
        fprintf(stderr, "Error: Matrix dimensions mismatch for addition.\n");
        exit(1);
    }
    MPSMatrix* c = matrix_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = a->data[i] + b->data[i];
    }
    return c;
}

static inline MPSMatrix* matrix_relu(MPSMatrix* a) {
    if (a == NULL) return NULL;
    MPSMatrix* c = matrix_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = a->data[i] > 0.0 ? a->data[i] : 0.0;
    }
    return c;
}

static inline MPSMatrix* matrix_sub(MPSMatrix* a, MPSMatrix* b) {
    if (a == NULL || b == NULL || a->rows != b->rows || a->cols != b->cols) {
        fprintf(stderr, "Error: Matrix dimensions mismatch for subtraction.\n");
        exit(1);
    }
    MPSMatrix* c = matrix_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = a->data[i] - b->data[i];
    }
    return c;
}

static inline MPSMatrix* matrix_scale(MPSMatrix* a, double scalar) {
    if (a == NULL) return NULL;
    MPSMatrix* c = matrix_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = a->data[i] * scalar;
    }
    return c;
}

static inline MPSMatrix* matrix_sigmoid(MPSMatrix* a) {
    if (a == NULL) return NULL;
    MPSMatrix* c = matrix_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = 1.0 / (1.0 + exp(-a->data[i]));
    }
    return c;
}

static inline MPSMatrix* matrix_softmax(MPSMatrix* a) {
    if (a == NULL) return NULL;
    MPSMatrix* c = matrix_new(a->rows, a->cols);
    for (int r = 0; r < a->rows; r++) {
        int offset = r * a->cols;
        double max_val = a->data[offset];
        for (int col = 1; col < a->cols; col++) {
            if (a->data[offset + col] > max_val) {
                max_val = a->data[offset + col];
            }
        }
        double sum = 0.0;
        for (int col = 0; col < a->cols; col++) {
            double ev = exp(a->data[offset + col] - max_val);
            c->data[offset + col] = ev;
            sum += ev;
        }
        for (int col = 0; col < a->cols; col++) {
            c->data[offset + col] /= (sum == 0.0 ? 1.0 : sum);
        }
    }
    return c;
}

static inline MPSMatrix* matrix_exp(MPSMatrix* a) {
    if (a == NULL) return NULL;
    MPSMatrix* c = matrix_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = exp(a->data[i]);
    }
    return c;
}

static inline MPSMatrix* matrix_log(MPSMatrix* a) {
    if (a == NULL) return NULL;
    MPSMatrix* c = matrix_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = log(a->data[i]);
    }
    return c;
}

/* --- float32 MPSMatrix32 operations --- */
static inline MPSMatrix32* matrix32_new(int rows, int cols) {
    MPSMatrix32* m = (MPSMatrix32*)malloc(sizeof(MPSMatrix32));
    m->rows = rows;
    m->cols = cols;
    m->data = (float*)malloc(rows * cols * sizeof(float));
    return m;
}

static inline void matrix32_free(MPSMatrix32* m) {
    if (m != NULL) {
        if (m->data != NULL) {
            free(m->data);
        }
        free(m);
    }
}

static inline float matrix32_get(MPSMatrix32* m, int r, int c) {
    if (m == NULL || r < 0 || r >= m->rows || c < 0 || c >= m->cols) {
        fprintf(stderr, "Index Error: Matrix32 subscript out of bounds.\n");
        exit(1);
    }
    return m->data[r * m->cols + c];
}

static inline void matrix32_set(MPSMatrix32* m, int r, int c, float val) {
    if (m == NULL || r < 0 || r >= m->rows || c < 0 || c >= m->cols) {
        fprintf(stderr, "Index Error: Matrix32 subscript out of bounds.\n");
        exit(1);
    }
    m->data[r * m->cols + c] = val;
}

static inline MPSMatrix32* matrix32_mul(MPSMatrix32* a, MPSMatrix32* b) {
    if (a == NULL || b == NULL || a->cols != b->rows) {
        fprintf(stderr, "Error: Matrix32 dimensions mismatch for multiplication (%dx%d vs %dx%d).\n", 
                a ? a->rows : 0, a ? a->cols : 0, b ? b->rows : 0, b ? b->cols : 0);
        exit(1);
    }
    MPSMatrix32* c = matrix32_new(a->rows, b->cols);
    
#ifdef MPS_USE_BLAS
    cblas_sgemm(CblasRowMajor, CblasNoTrans, CblasNoTrans,
                a->rows, b->cols, a->cols, 1.0f,
                a->data, a->cols, b->data, b->cols,
                0.0f, c->data, b->cols);
#else
    int b_size = b->rows * b->cols;
    float* b_T;
    float stack_buf[1024];
    if (b_size <= 1024) {
        b_T = stack_buf;
    } else {
        static THREAD_LOCAL float* tl_buf = NULL;
        static THREAD_LOCAL int tl_cap = 0;
        if (b_size > tl_cap) {
            tl_cap = b_size * 2;
            tl_buf = (float*)realloc(tl_buf, sizeof(float) * tl_cap);
        }
        b_T = tl_buf;
    }
    
    for (int r = 0; r < b->rows; r++) {
        for (int col = 0; col < b->cols; col++) {
            b_T[col * b->rows + r] = b->data[r * b->cols + col];
        }
    }
    
    for (int i = 0; i < a->rows; i++) {
        int a_offset = i * a->cols;
        int c_offset = i * c->cols;
        for (int j = 0; j < b->cols; j++) {
            float sum = 0.0f;
            int b_offset = j * b->rows;
            for (int k = 0; k < a->cols; k++) {
                sum += a->data[a_offset + k] * b_T[b_offset + k];
            }
            c->data[c_offset + j] = sum;
        }
    }
#endif
    return c;
}

static inline MPSMatrix32* matrix32_add(MPSMatrix32* a, MPSMatrix32* b) {
    if (a == NULL || b == NULL || a->rows != b->rows || a->cols != b->cols) {
        fprintf(stderr, "Error: Matrix32 dimensions mismatch for addition.\n");
        exit(1);
    }
    MPSMatrix32* c = matrix32_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = a->data[i] + b->data[i];
    }
    return c;
}

static inline MPSMatrix32* matrix32_relu(MPSMatrix32* a) {
    if (a == NULL) return NULL;
    MPSMatrix32* c = matrix32_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = a->data[i] > 0.0f ? a->data[i] : 0.0f;
    }
    return c;
}

static inline MPSMatrix32* matrix32_sub(MPSMatrix32* a, MPSMatrix32* b) {
    if (a == NULL || b == NULL || a->rows != b->rows || a->cols != b->cols) {
        fprintf(stderr, "Error: Matrix32 dimensions mismatch for subtraction.\n");
        exit(1);
    }
    MPSMatrix32* c = matrix32_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = a->data[i] - b->data[i];
    }
    return c;
}

static inline MPSMatrix32* matrix32_scale(MPSMatrix32* a, float scalar) {
    if (a == NULL) return NULL;
    MPSMatrix32* c = matrix32_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = a->data[i] * scalar;
    }
    return c;
}

static inline MPSMatrix32* matrix32_sigmoid(MPSMatrix32* a) {
    if (a == NULL) return NULL;
    MPSMatrix32* c = matrix32_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = 1.0f / (1.0f + expf(-a->data[i]));
    }
    return c;
}

static inline MPSMatrix32* matrix32_softmax(MPSMatrix32* a) {
    if (a == NULL) return NULL;
    MPSMatrix32* c = matrix32_new(a->rows, a->cols);
    for (int r = 0; r < a->rows; r++) {
        int offset = r * a->cols;
        float max_val = a->data[offset];
        for (int col = 1; col < a->cols; col++) {
            if (a->data[offset + col] > max_val) {
                max_val = a->data[offset + col];
            }
        }
        float sum = 0.0f;
        for (int col = 0; col < a->cols; col++) {
            float ev = expf(a->data[offset + col] - max_val);
            c->data[offset + col] = ev;
            sum += ev;
        }
        for (int col = 0; col < a->cols; col++) {
            c->data[offset + col] /= (sum == 0.0f ? 1.0f : sum);
        }
    }
    return c;
}

static inline MPSMatrix32* matrix32_exp(MPSMatrix32* a) {
    if (a == NULL) return NULL;
    MPSMatrix32* c = matrix32_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = expf(a->data[i]);
    }
    return c;
}

static inline MPSMatrix32* matrix32_log(MPSMatrix32* a) {
    if (a == NULL) return NULL;
    MPSMatrix32* c = matrix32_new(a->rows, a->cols);
    int size = a->rows * a->cols;
    for (int i = 0; i < size; i++) {
        c->data[i] = logf(a->data[i]);
    }
    return c;
}

/* --- Random Number Generation --- */
static THREAD_LOCAL uint64_t mps_rng_state = 0x853c49e6748fea9bULL;
static THREAD_LOCAL uint64_t mps_rng_inc = 0xda3e39cb94b95bdbULL;

static inline uint32_t mps_pcg32_next(void) {
    uint64_t oldstate = mps_rng_state;
    mps_rng_state = oldstate * 6364136223846793005ULL + mps_rng_inc;
    uint32_t xorshifted = (uint32_t)(((oldstate >> 18u) ^ oldstate) >> 27u);
    uint32_t rot = (uint32_t)(oldstate >> 59u);
    return (xorshifted >> rot) | (xorshifted << ((-rot) & 31));
}

static inline void mps_random_seed(int seed) {
    mps_rng_state = (uint64_t)seed + 1442695040888963407ULL;
    mps_rng_inc = 1442695040888963407ULL | 1;
    (void)mps_pcg32_next();
}

static inline double mps_random(void) {
    if (mps_rng_state == 0x853c49e6748fea9bULL) {
        mps_random_seed((int)time(NULL));
    }
    uint32_t val = mps_pcg32_next();
    return (double)val / 4294967296.0;
}

static inline int mps_randint(int min, int max) {
    if (mps_rng_state == 0x853c49e6748fea9bULL) {
        mps_random_seed((int)time(NULL));
    }
    if (min >= max) return min;
    uint32_t range = (uint32_t)(max - min + 1);
    uint32_t val = mps_pcg32_next();
    return min + (int)(val % range);
}

// Casting helpers needed by tensor functions
static inline int _to_int_str(const char* s) { return atoi(s); }
static inline int _to_int_double(double d) { return (int)d; }
static inline int _to_int_int(int i) { return i; }
static inline int _to_int_bool(bool b) { return b ? 1 : 0; }

#ifdef MPS_USE_PYTHON
static inline int _to_int_py(PyObject* o) {
    if (o == NULL) return 0;
    return (int)PyLong_AsLong(o);
}
#else
static inline int _to_int_py(PyObject* o) {
    if (o == NULL) return 0;
    if (o->type == OBJ_INT) return o->value.int_val;
    if (o->type == OBJ_FLOAT) return (int)o->value.float_val;
    if (o->type == OBJ_STRING) return atoi(o->value.string_val);
    if (o->type == OBJ_BOOL) return o->value.bool_val ? 1 : 0;
    return 0;
}
#endif

#define mps_to_int(X) _Generic((X), \
    const char*: _to_int_str, \
    char*: _to_int_str, \
    double: _to_int_double, \
    float: _to_int_double, \
    int: _to_int_int, \
    bool: _to_int_bool, \
    PyObject*: _to_int_py \
)(X)

/* --- N-Dimensional Tensor Definition and Helpers --- */
typedef struct {
    double* data;
    int ndim;
    int* shape;
    int* strides;
    int size;
} MPSTensor;

typedef struct {
    float* data;
    int ndim;
    int* shape;
    int* strides;
    int size;
} MPSTensor32;

static inline void parse_shape(PyObject* shape_obj, int* ndim_out, int** shape_out, int** strides_out, int* size_out) {
    int ndim = 0;
    int* shape = NULL;
    int* strides = NULL;
    int size = 1;

    if (shape_obj == NULL) {
        ndim = 1;
        shape = (int*)malloc(sizeof(int));
        shape[0] = 0;
        strides = (int*)malloc(sizeof(int));
        strides[0] = 1;
        size = 0;
    } else {
#ifdef MPS_USE_PYTHON
        if (PyLong_Check(shape_obj)) {
            ndim = 1;
            shape = (int*)malloc(sizeof(int));
            shape[0] = (int)PyLong_AsLong(shape_obj);
            size = shape[0];
        } else if (PyList_Check(shape_obj) || PyTuple_Check(shape_obj)) {
            ndim = (int)PySequence_Size(shape_obj);
            shape = (int*)malloc(ndim * sizeof(int));
            for (int i = 0; i < ndim; i++) {
                PyObject* item = PySequence_GetItem(shape_obj, i);
                shape[i] = (int)PyLong_AsLong(item);
                Py_DECREF(item);
                size *= shape[i];
            }
        }
#else
        if (shape_obj->type == OBJ_INT) {
            ndim = 1;
            shape = (int*)malloc(sizeof(int));
            shape[0] = shape_obj->value.int_val;
            size = shape[0];
        } else if (shape_obj->type == OBJ_LIST || shape_obj->type == OBJ_TUPLE) {
            ndim = shape_obj->value.list_val->size;
            shape = (int*)malloc(ndim * sizeof(int));
            for (int i = 0; i < ndim; i++) {
                PyObject* item = shape_obj->value.list_val->items[i];
                shape[i] = item->type == OBJ_INT ? item->value.int_val : 0;
                size *= shape[i];
            }
        }
#endif
        else {
            int val = mps_to_int(shape_obj);
            ndim = 1;
            shape = (int*)malloc(sizeof(int));
            shape[0] = val;
            size = val;
        }
        
        strides = (int*)malloc(ndim * sizeof(int));
        int current_stride = 1;
        for (int i = ndim - 1; i >= 0; i--) {
            strides[i] = current_stride;
            current_stride *= shape[i];
        }
    }

    *ndim_out = ndim;
    *shape_out = shape;
    *strides_out = strides;
    *size_out = size;
}

static inline MPSTensor* tensor_new(PyObject* shape_obj) {
    MPSTensor* t = (MPSTensor*)malloc(sizeof(MPSTensor));
    parse_shape(shape_obj, &t->ndim, &t->shape, &t->strides, &t->size);
    t->data = (double*)malloc(t->size * sizeof(double));
    for (int i = 0; i < t->size; i++) t->data[i] = 0.0;
    return t;
}

static inline MPSTensor32* tensor32_new(PyObject* shape_obj) {
    MPSTensor32* t = (MPSTensor32*)malloc(sizeof(MPSTensor32));
    parse_shape(shape_obj, &t->ndim, &t->shape, &t->strides, &t->size);
    t->data = (float*)malloc(t->size * sizeof(float));
    for (int i = 0; i < t->size; i++) t->data[i] = 0.0f;
    return t;
}

static inline void tensor_free(MPSTensor* t) {
    if (t != NULL) {
        if (t->shape != NULL) free(t->shape);
        if (t->strides != NULL) free(t->strides);
        if (t->data != NULL) free(t->data);
        free(t);
    }
}

static inline void tensor32_free(MPSTensor32* t) {
    if (t != NULL) {
        if (t->shape != NULL) free(t->shape);
        if (t->strides != NULL) free(t->strides);
        if (t->data != NULL) free(t->data);
        free(t);
    }
}

static inline PyObject* tensor_shape(MPSTensor* t) {
    if (t == NULL) return Py_None;
    PyObject* list = PyList_New(t->ndim);
    for (int i = 0; i < t->ndim; i++) {
        PyList_SetItem(list, i, to_py(t->shape[i]));
    }
    return list;
}

static inline PyObject* tensor_strides(MPSTensor* t) {
    if (t == NULL) return Py_None;
    PyObject* list = PyList_New(t->ndim);
    for (int i = 0; i < t->ndim; i++) {
        PyList_SetItem(list, i, to_py(t->strides[i]));
    }
    return list;
}

static inline PyObject* tensor32_shape(MPSTensor32* t) {
    if (t == NULL) return Py_None;
    PyObject* list = PyList_New(t->ndim);
    for (int i = 0; i < t->ndim; i++) {
        PyList_SetItem(list, i, to_py(t->shape[i]));
    }
    return list;
}

static inline PyObject* tensor32_strides(MPSTensor32* t) {
    if (t == NULL) return Py_None;
    PyObject* list = PyList_New(t->ndim);
    for (int i = 0; i < t->ndim; i++) {
        PyList_SetItem(list, i, to_py(t->strides[i]));
    }
    return list;
}

static inline int compute_tensor_offset(int ndim, int* shape, int* strides, PyObject* index_obj) {
    if (index_obj == NULL) return 0;
    int offset = 0;
    bool is_int = false;
    int int_val = 0;
    
#ifdef MPS_USE_PYTHON
    if (PyLong_Check(index_obj)) {
        is_int = true;
        int_val = (int)PyLong_AsLong(index_obj);
    }
#else
    if (index_obj->type == OBJ_INT) {
        is_int = true;
        int_val = index_obj->value.int_val;
    }
#endif
    
    if (is_int) {
        if (ndim == 1) {
            if (int_val < 0) int_val += shape[0];
            if (int_val < 0 || int_val >= shape[0]) {
                fprintf(stderr, "Index Error: Tensor index out of bounds.\n");
                exit(1);
            }
            return int_val * strides[0];
        } else {
            int total_size = 1;
            for (int i = 0; i < ndim; i++) total_size *= shape[i];
            if (int_val < 0) int_val += total_size;
            if (int_val < 0 || int_val >= total_size) {
                fprintf(stderr, "Index Error: Flat tensor index out of bounds.\n");
                exit(1);
            }
            int temp = int_val;
            for (int i = 0; i < ndim; i++) {
                int dim_idx = temp / strides[i];
                offset += dim_idx * strides[i];
                temp %= strides[i];
            }
            return offset;
        }
    }
    
    int index_len = 0;
#ifdef MPS_USE_PYTHON
    if (PyList_Check(index_obj) || PyTuple_Check(index_obj)) {
        index_len = (int)PySequence_Size(index_obj);
        for (int i = 0; i < index_len && i < ndim; i++) {
            PyObject* item = PySequence_GetItem(index_obj, i);
            int idx = (int)PyLong_AsLong(item);
            Py_DECREF(item);
            if (idx < 0) idx += shape[i];
            if (idx < 0 || idx >= shape[i]) {
                fprintf(stderr, "Index Error: Tensor index out of bounds.\n");
                exit(1);
            }
            offset += idx * strides[i];
        }
    }
#else
    if (index_obj->type == OBJ_LIST || index_obj->type == OBJ_TUPLE) {
        index_len = index_obj->value.list_val->size;
        for (int i = 0; i < index_len && i < ndim; i++) {
            PyObject* item = index_obj->value.list_val->items[i];
            int idx = item->type == OBJ_INT ? item->value.int_val : 0;
            if (idx < 0) idx += shape[i];
            if (idx < 0 || idx >= shape[i]) {
                fprintf(stderr, "Index Error: Tensor index out of bounds.\n");
                exit(1);
            }
            offset += idx * strides[i];
        }
    }
#endif
    else {
        int idx = mps_to_int(index_obj);
        int total_size = 1;
        for (int i = 0; i < ndim; i++) total_size *= shape[i];
        if (idx < 0) idx += total_size;
        if (idx < 0 || idx >= total_size) {
            fprintf(stderr, "Index Error: Tensor index out of bounds.\n");
            exit(1);
        }
        int temp = idx;
        for (int i = 0; i < ndim; i++) {
            int dim_idx = temp / strides[i];
            offset += dim_idx * strides[i];
            temp %= strides[i];
        }
    }
    
    return offset;
}

static inline double tensor_get(MPSTensor* t, PyObject* index_obj) {
    if (t == NULL) return 0.0;
    int offset = compute_tensor_offset(t->ndim, t->shape, t->strides, index_obj);
    return t->data[offset];
}

static inline void tensor_set(MPSTensor* t, PyObject* index_obj, double val) {
    if (t == NULL) return;
    int offset = compute_tensor_offset(t->ndim, t->shape, t->strides, index_obj);
    t->data[offset] = val;
}

static inline float tensor32_get(MPSTensor32* t, PyObject* index_obj) {
    if (t == NULL) return 0.0f;
    int offset = compute_tensor_offset(t->ndim, t->shape, t->strides, index_obj);
    return t->data[offset];
}

static inline void tensor32_set(MPSTensor32* t, PyObject* index_obj, float val) {
    if (t == NULL) return;
    int offset = compute_tensor_offset(t->ndim, t->shape, t->strides, index_obj);
    t->data[offset] = val;
}

static inline bool broadcast_shapes(int ndim_a, int* shape_a, int ndim_b, int* shape_b, int* ndim_out, int** shape_out) {
    int ndim = ndim_a > ndim_b ? ndim_a : ndim_b;
    int* shape = (int*)malloc(ndim * sizeof(int));
    
    for (int i = 0; i < ndim; i++) {
        int dim_a = (ndim_a - 1 - i >= 0) ? shape_a[ndim_a - 1 - i] : 1;
        int dim_b = (ndim_b - 1 - i >= 0) ? shape_b[ndim_b - 1 - i] : 1;
        
        if (dim_a != dim_b && dim_a != 1 && dim_b != 1) {
            free(shape);
            return false;
        }
        shape[ndim - 1 - i] = dim_a > dim_b ? dim_a : dim_b;
    }
    
    *ndim_out = ndim;
    *shape_out = shape;
    return true;
}

static inline int get_broadcast_offset(int ndim_out, int* out_idx, int ndim_in, int* shape_in, int* strides_in) {
    int offset = 0;
    for (int i = 0; i < ndim_in; i++) {
        int out_dim_idx = ndim_out - 1 - (ndim_in - 1 - i);
        int idx = out_idx[out_dim_idx];
        if (shape_in[i] == 1) {
            idx = 0;
        }
        offset += idx * strides_in[i];
    }
    return offset;
}

static inline void flat_to_multi(int ndim, int* shape, int* strides, int flat_idx, int* out_idx) {
    int temp = flat_idx;
    for (int i = 0; i < ndim; i++) {
        out_idx[i] = temp / strides[i];
        temp %= strides[i];
    }
}

static inline MPSTensor* tensor_broadcast_op(MPSTensor* a, MPSTensor* b, char op) {
    if (a == NULL || b == NULL) return NULL;
    int ndim_out;
    int* shape_out;
    if (!broadcast_shapes(a->ndim, a->shape, b->ndim, b->shape, &ndim_out, &shape_out)) {
        fprintf(stderr, "Error: Tensor dimensions mismatch for broadcasting.\n");
        exit(1);
    }
    
    PyObject* shape_list = PyList_New(ndim_out);
    for (int i = 0; i < ndim_out; i++) {
        PyList_SetItem(shape_list, i, to_py(shape_out[i]));
    }
    
    MPSTensor* c = tensor_new(shape_list);
    Py_DECREF(shape_list);
    free(shape_out);
    
    int* out_idx = (int*)malloc(ndim_out * sizeof(int));
    for (int i = 0; i < c->size; i++) {
        flat_to_multi(c->ndim, c->shape, c->strides, i, out_idx);
        int offset_a = get_broadcast_offset(c->ndim, out_idx, a->ndim, a->shape, a->strides);
        int offset_b = get_broadcast_offset(c->ndim, out_idx, b->ndim, b->shape, b->strides);
        
        double val_a = a->data[offset_a];
        double val_b = b->data[offset_b];
        double res = 0.0;
        if (op == '+') res = val_a + val_b;
        else if (op == '-') res = val_a - val_b;
        else if (op == '*') res = val_a * val_b;
        else if (op == '/') res = val_b != 0.0 ? val_a / val_b : 0.0;
        
        c->data[i] = res;
    }
    free(out_idx);
    return c;
}

static inline MPSTensor* tensor_add(MPSTensor* a, MPSTensor* b) { return tensor_broadcast_op(a, b, '+'); }
static inline MPSTensor* tensor_sub(MPSTensor* a, MPSTensor* b) { return tensor_broadcast_op(a, b, '-'); }
static inline MPSTensor* tensor_mul(MPSTensor* a, MPSTensor* b) { return tensor_broadcast_op(a, b, '*'); }
static inline MPSTensor* tensor_div(MPSTensor* a, MPSTensor* b) { return tensor_broadcast_op(a, b, '/'); }

static inline MPSTensor32* tensor32_broadcast_op(MPSTensor32* a, MPSTensor32* b, char op) {
    if (a == NULL || b == NULL) return NULL;
    int ndim_out;
    int* shape_out;
    if (!broadcast_shapes(a->ndim, a->shape, b->ndim, b->shape, &ndim_out, &shape_out)) {
        fprintf(stderr, "Error: Tensor32 dimensions mismatch for broadcasting.\n");
        exit(1);
    }
    
    PyObject* shape_list = PyList_New(ndim_out);
    for (int i = 0; i < ndim_out; i++) {
        PyList_SetItem(shape_list, i, to_py(shape_out[i]));
    }
    
    MPSTensor32* c = tensor32_new(shape_list);
    Py_DECREF(shape_list);
    free(shape_out);
    
    int* out_idx = (int*)malloc(ndim_out * sizeof(int));
    for (int i = 0; i < c->size; i++) {
        flat_to_multi(c->ndim, c->shape, c->strides, i, out_idx);
        int offset_a = get_broadcast_offset(c->ndim, out_idx, a->ndim, a->shape, a->strides);
        int offset_b = get_broadcast_offset(c->ndim, out_idx, b->ndim, b->shape, b->strides);
        
        float val_a = a->data[offset_a];
        float val_b = b->data[offset_b];
        float res = 0.0f;
        if (op == '+') res = val_a + val_b;
        else if (op == '-') res = val_a - val_b;
        else if (op == '*') res = val_a * val_b;
        else if (op == '/') res = val_b != 0.0f ? val_a / val_b : 0.0f;
        
        c->data[i] = res;
    }
    free(out_idx);
    return c;
}

static inline MPSTensor32* tensor32_add(MPSTensor32* a, MPSTensor32* b) { return tensor32_broadcast_op(a, b, '+'); }
static inline MPSTensor32* tensor32_sub(MPSTensor32* a, MPSTensor32* b) { return tensor32_broadcast_op(a, b, '-'); }
static inline MPSTensor32* tensor32_mul(MPSTensor32* a, MPSTensor32* b) { return tensor32_broadcast_op(a, b, '*'); }
static inline MPSTensor32* tensor32_div(MPSTensor32* a, MPSTensor32* b) { return tensor32_broadcast_op(a, b, '/'); }

static inline MPSTensor* tensor_sigmoid(MPSTensor* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor_shape(a);
    MPSTensor* c = tensor_new(shape_list);
    Py_DECREF(shape_list);
    for (int i = 0; i < a->size; i++) {
        c->data[i] = 1.0 / (1.0 + exp(-a->data[i]));
    }
    return c;
}

static inline MPSTensor* tensor_relu(MPSTensor* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor_shape(a);
    MPSTensor* c = tensor_new(shape_list);
    Py_DECREF(shape_list);
    for (int i = 0; i < a->size; i++) {
        c->data[i] = a->data[i] > 0.0 ? a->data[i] : 0.0;
    }
    return c;
}

static inline MPSTensor* tensor_exp(MPSTensor* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor_shape(a);
    MPSTensor* c = tensor_new(shape_list);
    Py_DECREF(shape_list);
    for (int i = 0; i < a->size; i++) {
        c->data[i] = exp(a->data[i]);
    }
    return c;
}

static inline MPSTensor* tensor_log(MPSTensor* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor_shape(a);
    MPSTensor* c = tensor_new(shape_list);
    Py_DECREF(shape_list);
    for (int i = 0; i < a->size; i++) {
        c->data[i] = log(a->data[i]);
    }
    return c;
}

static inline MPSTensor* tensor_softmax(MPSTensor* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor_shape(a);
    MPSTensor* c = tensor_new(shape_list);
    Py_DECREF(shape_list);
    
    int last_dim_size = a->shape[a->ndim - 1];
    int num_slices = a->size / last_dim_size;
    
    for (int s = 0; s < num_slices; s++) {
        int offset = s * last_dim_size;
        double max_val = a->data[offset];
        for (int i = 1; i < last_dim_size; i++) {
            if (a->data[offset + i] > max_val) {
                max_val = a->data[offset + i];
            }
        }
        double sum = 0.0;
        for (int i = 0; i < last_dim_size; i++) {
            double ev = exp(a->data[offset + i] - max_val);
            c->data[offset + i] = ev;
            sum += ev;
        }
        for (int i = 0; i < last_dim_size; i++) {
            c->data[offset + i] /= (sum == 0.0 ? 1.0 : sum);
        }
    }
    return c;
}

static inline MPSTensor32* tensor32_sigmoid(MPSTensor32* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor32_shape(a);
    MPSTensor32* c = tensor32_new(shape_list);
    Py_DECREF(shape_list);
    for (int i = 0; i < a->size; i++) {
        c->data[i] = 1.0f / (1.0f + expf(-a->data[i]));
    }
    return c;
}

static inline MPSTensor32* tensor32_relu(MPSTensor32* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor32_shape(a);
    MPSTensor32* c = tensor32_new(shape_list);
    Py_DECREF(shape_list);
    for (int i = 0; i < a->size; i++) {
        c->data[i] = a->data[i] > 0.0f ? a->data[i] : 0.0f;
    }
    return c;
}

static inline MPSTensor32* tensor32_exp(MPSTensor32* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor32_shape(a);
    MPSTensor32* c = tensor32_new(shape_list);
    Py_DECREF(shape_list);
    for (int i = 0; i < a->size; i++) {
        c->data[i] = expf(a->data[i]);
    }
    return c;
}

static inline MPSTensor32* tensor32_log(MPSTensor32* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor32_shape(a);
    MPSTensor32* c = tensor32_new(shape_list);
    Py_DECREF(shape_list);
    for (int i = 0; i < a->size; i++) {
        c->data[i] = logf(a->data[i]);
    }
    return c;
}

static inline MPSTensor32* tensor32_softmax(MPSTensor32* a) {
    if (a == NULL) return NULL;
    PyObject* shape_list = tensor32_shape(a);
    MPSTensor32* c = tensor32_new(shape_list);
    Py_DECREF(shape_list);
    
    int last_dim_size = a->shape[a->ndim - 1];
    int num_slices = a->size / last_dim_size;
    
    for (int s = 0; s < num_slices; s++) {
        int offset = s * last_dim_size;
        float max_val = a->data[offset];
        for (int i = 1; i < last_dim_size; i++) {
            if (a->data[offset + i] > max_val) {
                max_val = a->data[offset + i];
            }
        }
        float sum = 0.0f;
        for (int i = 0; i < last_dim_size; i++) {
            float ev = expf(a->data[offset + i] - max_val);
            c->data[offset + i] = ev;
            sum += ev;
        }
        for (int i = 0; i < last_dim_size; i++) {
            c->data[offset + i] /= (sum == 0.0f ? 1.0f : sum);
        }
    }
    return c;
}

/* --- Printing Utilities --- */
static inline void _print_string(const char* s) {
    printf("%s\n", s);
}

static inline void _print_int(int i) {
    printf("%d\n", i);
}

static inline void _print_double(double d) {
    printf("%g\n", d);
}

static inline void _print_bool(bool b) {
    printf("%s\n", b ? "true" : "false");
}

#ifdef MPS_USE_PYTHON
static inline void _print_pyobject(PyObject* obj) {
    if (obj == NULL) {
        printf("None\n");
        return;
    }
    PyObject* repr = PyObject_Repr(obj);
    if (repr != NULL) {
        PyObject* str = PyUnicode_AsUTF8String(repr);
        if (str != NULL) {
            printf("%s\n", PyBytes_AsString(str));
            Py_DECREF(str);
        }
        Py_DECREF(repr);
    }
}
#else
static inline void _print_pyobject_raw(PyObject* obj) {
    if (obj == NULL) {
        printf("null");
        return;
    }
    switch (obj->type) {
        case OBJ_INT: printf("%d", obj->value.int_val); break;
        case OBJ_FLOAT: printf("%g", obj->value.float_val); break;
        case OBJ_STRING: printf("%s", obj->value.string_val); break;
        case OBJ_BOOL: printf("%s", obj->value.bool_val ? "true" : "false"); break;
        case OBJ_LIST: {
            printf("[");
            for (int i = 0; i < obj->value.list_val->size; i++) {
                _print_pyobject_raw(obj->value.list_val->items[i]);
                if (i < obj->value.list_val->size - 1) printf(", ");
            }
            printf("]");
            break;
        }
        case OBJ_TUPLE: {
            printf("(");
            for (int i = 0; i < obj->value.list_val->size; i++) {
                _print_pyobject_raw(obj->value.list_val->items[i]);
                if (i < obj->value.list_val->size - 1) printf(", ");
            }
            printf(")");
            break;
        }
        case OBJ_DICT: {
            printf("{");
            for (int i = 0; i < obj->value.dict_val->size; i++) {
                _print_pyobject_raw(obj->value.dict_val->pairs[i].key);
                printf(": ");
                _print_pyobject_raw(obj->value.dict_val->pairs[i].val);
                if (i < obj->value.dict_val->size - 1) printf(", ");
            }
            printf("}");
            break;
        }
        case OBJ_ITER: printf("<iterator>"); break;
        case OBJ_SLICE: {
            printf("slice(");
            _print_pyobject_raw(obj->value.slice_val->start);
            printf(", ");
            _print_pyobject_raw(obj->value.slice_val->stop);
            printf(", ");
            _print_pyobject_raw(obj->value.slice_val->step);
            printf(")");
            break;
        }
        case OBJ_NULL: printf("null"); break;
    }
}

static inline void _print_pyobject(PyObject* obj) {
    _print_pyobject_raw(obj);
    printf("\n");
}
#endif /* MPS_USE_PYTHON */

static inline void _print_matrix(MPSMatrix* m) {
    if (m == NULL) {
        printf("Matrix: NULL\n");
        return;
    }
    printf("Matrix (%dx%d):\n", m->rows, m->cols);
    int max_rows = m->rows > 10 ? 10 : m->rows;
    int max_cols = m->cols > 10 ? 10 : m->cols;
    for (int i = 0; i < max_rows; i++) {
        printf("  [ ");
        for (int j = 0; j < max_cols; j++) {
            printf("%g ", m->data[i * m->cols + j]);
        }
        if (m->cols > 10) printf("... ");
        printf("]\n");
    }
    if (m->rows > 10) {
        printf("  ...\n");
    }
}

static inline void _print_matrix32(MPSMatrix32* m) {
    if (m == NULL) {
        printf("Matrix32: NULL\n");
        return;
    }
    printf("Matrix32 (%dx%d):\n", m->rows, m->cols);
    int max_rows = m->rows > 10 ? 10 : m->rows;
    int max_cols = m->cols > 10 ? 10 : m->cols;
    for (int i = 0; i < max_rows; i++) {
        printf("  [ ");
        for (int j = 0; j < max_cols; j++) {
            printf("%g ", (double)m->data[i * m->cols + j]);
        }
        if (m->cols > 10) printf("... ");
        printf("]\n");
    }
    if (m->rows > 10) {
        printf("  ...\n");
    }
}

static inline void _print_tensor(MPSTensor* t) {
    if (t == NULL) {
        printf("Tensor: NULL\n");
        return;
    }
    printf("Tensor (ndim=%d, size=%d): shape=[", t->ndim, t->size);
    for (int i = 0; i < t->ndim; i++) {
        printf("%d%s", t->shape[i], i < t->ndim - 1 ? ", " : "");
    }
    printf("]\n");
    printf("  [ ");
    int max_elements = t->size > 10 ? 10 : t->size;
    for (int i = 0; i < max_elements; i++) {
        printf("%g ", t->data[i]);
    }
    if (t->size > 10) printf("... ");
    printf("]\n");
}

static inline void _print_tensor32(MPSTensor32* t) {
    if (t == NULL) {
        printf("Tensor32: NULL\n");
        return;
    }
    printf("Tensor32 (ndim=%d, size=%d): shape=[", t->ndim, t->size);
    for (int i = 0; i < t->ndim; i++) {
        printf("%d%s", t->shape[i], i < t->ndim - 1 ? ", " : "");
    }
    printf("]\n");
    printf("  [ ");
    int max_elements = t->size > 10 ? 10 : t->size;
    for (int i = 0; i < max_elements; i++) {
        printf("%g ", (double)t->data[i]);
    }
    if (t->size > 10) printf("... ");
    printf("]\n");
}

#define print(X) _Generic((X), \
    const char*: _print_string, \
    char*: _print_string, \
    int: _print_int, \
    double: _print_double, \
    float: _print_double, \
    bool: _print_bool, \
    PyObject*: _print_pyobject, \
    MPSMatrix*: _print_matrix, \
    MPSMatrix32*: _print_matrix32, \
    MPSTensor*: _print_tensor, \
    MPSTensor32*: _print_tensor32 \
)(X)

#define mps_print(X) print(X)
#define mps_println(X) print(X)

/* --- CPython Variadic FFI helper --- */
#ifdef MPS_USE_PYTHON
static inline PyObject* py_call(PyObject* obj, const char* name, int argc, ...) {
    if (obj == NULL) return NULL;
    
    // Intercept standard MPS OOP collection methods that Python lists/dicts don't natively expose
    if (PyList_Check(obj)) {
        if (strcmp(name, "length") == 0) {
            return PyLong_FromSsize_t(PyList_Size(obj));
        }
    } else if (PyDict_Check(obj)) {
        if (strcmp(name, "contains") == 0 && argc == 1) {
            va_list ap;
            va_start(ap, argc);
            PyObject* key = va_arg(ap, PyObject*);
            va_end(ap);
            int res = PyDict_Contains(obj, key);
            return PyBool_FromLong(res > 0);
        } else if (strcmp(name, "length") == 0) {
            return PyLong_FromSsize_t(PyDict_Size(obj));
        }
    }

    PyObject* func = PyObject_GetAttrString(obj, name);
    if (func == NULL || !PyCallable_Check(func)) {
        Py_XDECREF(func);
        return NULL;
    }
    
    PyObject* args = PyTuple_New(argc);
    PyObject* temp_args[16];
    int max_args = argc < 16 ? argc : 16;
    va_list ap;
    va_start(ap, argc);
    for (int i = 0; i < max_args; i++) {
        PyObject* arg = va_arg(ap, PyObject*);
        temp_args[i] = arg;
        Py_XINCREF(arg);
        PyTuple_SetItem(args, i, arg); // Steals the reference
    }
    va_end(ap);
    
    PyObject* res = PyObject_CallObject(func, args);
    Py_DECREF(func);
    Py_DECREF(args);
    for (int i = 0; i < max_args; i++) {
        Py_XDECREF(temp_args[i]);
    }
    return res;
}
#endif /* MPS_USE_PYTHON */

/* --- Standard Library Functions --- */
static inline const char* mps_input(const char* prompt) {
    printf("%s", prompt);
    static THREAD_LOCAL char buf[1024];
    if (fgets(buf, sizeof(buf), stdin) == NULL) return "";
    size_t len = strlen(buf);
    if (len > 0 && buf[len - 1] == '\n') buf[len - 1] = '\0';
    return buf;
}



static inline double _to_float_str(const char* s) { return atof(s); }
static inline double _to_float_double(double d) { return d; }
static inline double _to_float_int(int i) { return (double)i; }
static inline double _to_float_bool(bool b) { return b ? 1.0 : 0.0; }

#ifdef MPS_USE_PYTHON
static inline double _to_float_py(PyObject* o) {
    if (o == NULL) return 0.0;
    return PyFloat_AsDouble(o);
}
#else
static inline double _to_float_py(PyObject* o) {
    if (o == NULL) return 0.0;
    if (o->type == OBJ_INT) return (double)o->value.int_val;
    if (o->type == OBJ_FLOAT) return o->value.float_val;
    if (o->type == OBJ_STRING) return atof(o->value.string_val);
    if (o->type == OBJ_BOOL) return o->value.bool_val ? 1.0 : 0.0;
    return 0.0;
}
#endif

#define mps_to_float(X) _Generic((X), \
    const char*: _to_float_str, \
    char*: _to_float_str, \
    double: _to_float_double, \
    float: _to_float_double, \
    int: _to_float_int, \
    bool: _to_float_bool, \
    PyObject*: _to_float_py \
)(X)

static inline const char* _to_string_str(const char* s) { return s; }
static inline const char* _to_string_double(double d) {
    static THREAD_LOCAL char buf[64];
    sprintf(buf, "%g", d);
    return buf;
}
static inline const char* _to_string_int(int i) {
    static THREAD_LOCAL char buf[32];
    sprintf(buf, "%d", i);
    return buf;
}
static inline const char* _to_string_bool(bool b) { return b ? "true" : "false"; }

#ifdef MPS_USE_PYTHON
static inline const char* _to_string_py(PyObject* o) {
    if (o == NULL) return "None";
    PyObject* repr = PyObject_Repr(o);
    if (repr != NULL) {
        PyObject* str = PyUnicode_AsUTF8String(repr);
        if (str != NULL) {
            const char* s = PyBytes_AsString(str);
            static THREAD_LOCAL char buf[1024];
            strncpy(buf, s, sizeof(buf));
            buf[sizeof(buf) - 1] = '\0';
            Py_DECREF(str);
            Py_DECREF(repr);
            return buf;
        }
        Py_DECREF(repr);
    }
    return "None";
}
#else
static inline const char* _to_string_py(PyObject* o) {
    if (o == NULL) return "null";
    static THREAD_LOCAL char buf[1024];
    switch (o->type) {
        case OBJ_INT: sprintf(buf, "%d", o->value.int_val); break;
        case OBJ_FLOAT: sprintf(buf, "%g", o->value.float_val); break;
        case OBJ_STRING: return o->value.string_val;
        case OBJ_BOOL: return o->value.bool_val ? "true" : "false";
        default: sprintf(buf, "<object>"); break;
    }
    return buf;
}
#endif

#define mps_to_string(X) _Generic((X), \
    const char*: _to_string_str, \
    char*: _to_string_str, \
    double: _to_string_double, \
    float: _to_string_double, \
    int: _to_string_int, \
    bool: _to_string_bool, \
    PyObject*: _to_string_py \
)(X)

static inline bool _to_bool_str(const char* s) { return strlen(s) > 0; }
static inline bool _to_bool_double(double d) { return d != 0.0; }
static inline bool _to_bool_int(int i) { return i != 0; }
static inline bool _to_bool_bool(bool b) { return b; }

#ifdef MPS_USE_PYTHON
static inline bool _to_bool_py(PyObject* o) {
    if (o == NULL) return false;
    return PyObject_IsTrue(o) == 1;
}
#else
static inline bool _to_bool_py(PyObject* o) {
    if (o == NULL) return false;
    if (o->type == OBJ_INT) return o->value.int_val != 0;
    if (o->type == OBJ_FLOAT) return o->value.float_val != 0.0;
    if (o->type == OBJ_STRING) return strlen(o->value.string_val) > 0;
    if (o->type == OBJ_BOOL) return o->value.bool_val;
    return true;
}
#endif

#define mps_to_bool(X) _Generic((X), \
    const char*: _to_bool_str, \
    char*: _to_bool_str, \
    double: _to_bool_double, \
    float: _to_bool_double, \
    int: _to_bool_int, \
    bool: _to_bool_bool, \
    PyObject*: _to_bool_py \
)(X)

// Math
#define mps_abs(X) fabs(X)
#define mps_sqrt(X) sqrt(X)
#define mps_pow(B, E) pow(B, E)
#define mps_floor(X) ((int)floor(X))
#define mps_ceil(X) ((int)ceil(X))
#define mps_round(X) ((int)round(X))
#define mps_min(A, B) ((A) < (B) ? (A) : (B))
#define mps_max(A, B) ((A) > (B) ? (A) : (B))
#define mps_clamp(V, LO, HI) ((V) < (LO) ? (LO) : ((V) > (HI) ? (HI) : (V)))
#define mps_sin(X) sin(X)
#define mps_cos(X) cos(X)
#define mps_tan(X) tan(X)

// File IO
static inline const char* mps_file_read(const char* filename) {
    FILE* f = fopen(filename, "rb");
    if (f == NULL) return "";
    fseek(f, 0, SEEK_END);
    long len = ftell(f);
    fseek(f, 0, SEEK_SET);
    char* buf = (char*)malloc(len + 1);
    long read_bytes = fread(buf, 1, len, f);
    buf[read_bytes] = '\0';
    fclose(f);
    
    #define MPS_STR_POOL_SIZE 8
    static THREAD_LOCAL char* mps_str_pool[MPS_STR_POOL_SIZE];
    static THREAD_LOCAL int mps_str_pool_idx = 0;
    
    int idx = mps_str_pool_idx;
    if (mps_str_pool[idx] != NULL) free(mps_str_pool[idx]);
    mps_str_pool[idx] = buf;
    mps_str_pool_idx = (idx + 1) % MPS_STR_POOL_SIZE;
    return buf;
}

static inline void mps_file_write(const char* filename, const char* content) {
    FILE* f = fopen(filename, "wb");
    if (f != NULL) {
        fputs(content, f);
        fclose(f);
    }
}

static inline void mps_file_append(const char* filename, const char* content) {
    FILE* f = fopen(filename, "ab");
    if (f != NULL) {
        fputs(content, f);
        fclose(f);
    }
}

static inline bool mps_file_exists(const char* filename) {
    FILE* f = fopen(filename, "rb");
    if (f != NULL) {
        fclose(f);
        return true;
    }
    return false;
}

static inline char* mps_strdup(const char* s) {
    if (s == NULL) return NULL;
    size_t len = strlen(s);
    char* copy = (char*)malloc(len + 1);
    if (copy != NULL) {
        memcpy(copy, s, len + 1);
    }
    return copy;
}

static inline const char* mps_str_get_char(const char* s, int index) {
    if (s == NULL) return "";
    int len = (int)strlen(s);
    if (index < 0) {
        index = len + index;
    }
    if (index < 0 || index >= len) {
        fprintf(stderr, "Index Error: String index out of range.\n");
        exit(1);
    }
    char* buf = (char*)malloc(2);
    buf[0] = s[index];
    buf[1] = '\0';
    
    #define MPS_CHAR_POOL_SIZE 8
    static THREAD_LOCAL char* _char_pool[MPS_CHAR_POOL_SIZE];
    static THREAD_LOCAL int _char_idx = 0;
    int idx = _char_idx;
    if (_char_pool[idx] != NULL) free(_char_pool[idx]);
    _char_pool[idx] = buf;
    _char_idx = (idx + 1) % MPS_CHAR_POOL_SIZE;
    return buf;
}

static inline int mps_str_len(const char* s) {
    if (s == NULL) return 0;
    return (int)strlen(s);
}

/* --- String Concat (for + operator on strings) --- */
#define MPS_CONCAT_POOL_SIZE 16
static inline const char* mps_str_concat(const char* a, const char* b) {
    if (a == NULL) a = "";
    if (b == NULL) b = "";
    size_t la = strlen(a);
    size_t lb = strlen(b);
    char* buf = (char*)malloc(la + lb + 1);
    memcpy(buf, a, la);
    memcpy(buf + la, b, lb);
    buf[la + lb] = '\0';

    static THREAD_LOCAL char* _concat_pool[MPS_CONCAT_POOL_SIZE];
    static THREAD_LOCAL int _concat_idx = 0;
    int idx = _concat_idx;
    if (_concat_pool[idx] != NULL) free(_concat_pool[idx]);
    _concat_pool[idx] = buf;
    _concat_idx = (idx + 1) % MPS_CONCAT_POOL_SIZE;
    return buf;
}

/* --- Extended String Functions --- */
static inline const char* mps_str_upper(const char* s) {
    if (s == NULL) return "";
    size_t len = strlen(s);
    char* buf = (char*)malloc(len + 1);
    for (size_t i = 0; i < len; i++) {
        buf[i] = (s[i] >= 'a' && s[i] <= 'z') ? (s[i] - 32) : s[i];
    }
    buf[len] = '\0';
    static THREAD_LOCAL char* _upper_pool[4];
    static THREAD_LOCAL int _upper_idx = 0;
    int idx = _upper_idx;
    if (_upper_pool[idx] != NULL) free(_upper_pool[idx]);
    _upper_pool[idx] = buf;
    _upper_idx = (idx + 1) % 4;
    return buf;
}

static inline const char* mps_str_lower(const char* s) {
    if (s == NULL) return "";
    size_t len = strlen(s);
    char* buf = (char*)malloc(len + 1);
    for (size_t i = 0; i < len; i++) {
        buf[i] = (s[i] >= 'A' && s[i] <= 'Z') ? (s[i] + 32) : s[i];
    }
    buf[len] = '\0';
    static THREAD_LOCAL char* _lower_pool[4];
    static THREAD_LOCAL int _lower_idx = 0;
    int idx = _lower_idx;
    if (_lower_pool[idx] != NULL) free(_lower_pool[idx]);
    _lower_pool[idx] = buf;
    _lower_idx = (idx + 1) % 4;
    return buf;
}

static inline const char* mps_str_trim(const char* s) {
    if (s == NULL) return "";
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') s++;
    size_t len = strlen(s);
    while (len > 0 && (s[len-1] == ' ' || s[len-1] == '\t' || s[len-1] == '\n' || s[len-1] == '\r')) len--;
    char* buf = (char*)malloc(len + 1);
    memcpy(buf, s, len);
    buf[len] = '\0';
    static THREAD_LOCAL char* _trim_pool[4];
    static THREAD_LOCAL int _trim_idx = 0;
    int idx = _trim_idx;
    if (_trim_pool[idx] != NULL) free(_trim_pool[idx]);
    _trim_pool[idx] = buf;
    _trim_idx = (idx + 1) % 4;
    return buf;
}

static inline bool mps_str_contains(const char* s, const char* sub) {
    if (s == NULL || sub == NULL) return false;
    return strstr(s, sub) != NULL;
}

static inline bool mps_str_starts_with(const char* s, const char* prefix) {
    if (s == NULL || prefix == NULL) return false;
    return strncmp(s, prefix, strlen(prefix)) == 0;
}

static inline bool mps_str_ends_with(const char* s, const char* suffix) {
    if (s == NULL || suffix == NULL) return false;
    size_t sl = strlen(s);
    size_t pl = strlen(suffix);
    if (pl > sl) return false;
    return strcmp(s + sl - pl, suffix) == 0;
}

static inline const char* mps_str_replace(const char* s, const char* old_str, const char* new_str) {
    if (s == NULL || old_str == NULL || new_str == NULL) return s ? s : "";
    size_t old_len = strlen(old_str);
    size_t new_len = strlen(new_str);
    if (old_len == 0) return s;

    /* Count occurrences */
    int count = 0;
    const char* tmp = s;
    while ((tmp = strstr(tmp, old_str)) != NULL) { count++; tmp += old_len; }
    if (count == 0) return s;

    size_t result_len = strlen(s) + count * (new_len - old_len);
    char* buf = (char*)malloc(result_len + 1);
    char* out = buf;
    while (*s) {
        if (strncmp(s, old_str, old_len) == 0) {
            memcpy(out, new_str, new_len);
            out += new_len;
            s += old_len;
        } else {
            *out++ = *s++;
        }
    }
    *out = '\0';
    static THREAD_LOCAL char* _replace_pool[4];
    static THREAD_LOCAL int _replace_idx = 0;
    int idx = _replace_idx;
    if (_replace_pool[idx] != NULL) free(_replace_pool[idx]);
    _replace_pool[idx] = buf;
    _replace_idx = (idx + 1) % 4;
    return buf;
}

#ifdef MPS_USE_PYTHON
static inline PyObject* mps_str_split(const char* s, const char* sep) {
    PyObject* py_s = PyUnicode_FromString(s);
    PyObject* py_sep = PyUnicode_FromString(sep);
    PyObject* py_list = PyObject_CallMethod(py_s, "split", "O", py_sep);
    Py_DECREF(py_s);
    Py_DECREF(py_sep);
    return py_list;
}

static inline const char* mps_str_join(const char* connector, PyObject* list) {
    PyObject* py_connector = PyUnicode_FromString(connector);
    PyObject* py_joined = PyObject_CallMethod(py_connector, "join", "O", list);
    const char* res = PyUnicode_AsUTF8(py_joined);
    const char* pooled = mps_str_concat(res, "");
    Py_DECREF(py_connector);
    Py_XDECREF(py_joined);
    return pooled;
}
#else
static inline PyObject* mps_str_split(const char* s, const char* sep) {
    PyObject* list = PyList_New(0);
    if (s == NULL) return list;
    if (sep == NULL || strlen(sep) == 0) sep = " ";
    
    char* copy = mps_strdup(s);
    size_t sep_len = strlen(sep);
    char* token = copy;
    char* next;
    
    while ((next = strstr(token, sep)) != NULL) {
        *next = '\0';
        PyList_Append(list, _to_py_string(mps_strdup(token)));
        token = next + sep_len;
    }
    PyList_Append(list, _to_py_string(mps_strdup(token)));
    free(copy);
    return list;
}

static inline const char* mps_str_join(const char* connector, PyObject* list) {
    if (list == NULL || (list->type != OBJ_LIST && list->type != OBJ_TUPLE)) return "";
    MPS_ListImpl* impl = list->value.list_val;
    if (impl->size == 0) return "";
    
    size_t conn_len = strlen(connector);
    size_t total_len = 0;
    for (int i = 0; i < impl->size; i++) {
        PyObject* item = impl->items[i];
        if (item != NULL && item->type == OBJ_STRING) {
            total_len += strlen(item->value.string_val);
        }
        if (i < impl->size - 1) {
            total_len += conn_len;
        }
    }
    
    char* buf = (char*)malloc(total_len + 1);
    char* p = buf;
    for (int i = 0; i < impl->size; i++) {
        PyObject* item = impl->items[i];
        if (item != NULL && item->type == OBJ_STRING) {
            size_t len = strlen(item->value.string_val);
            memcpy(p, item->value.string_val, len);
            p += len;
        }
        if (i < impl->size - 1) {
            memcpy(p, connector, conn_len);
            p += conn_len;
        }
    }
    *p = '\0';
    
    static THREAD_LOCAL char* _join_pool[4];
    static THREAD_LOCAL int _join_idx = 0;
    int idx = _join_idx;
    if (_join_pool[idx] != NULL) free(_join_pool[idx]);
    _join_pool[idx] = buf;
    _join_idx = (idx + 1) % 4;
    return buf;
}
#endif

/* --- Math Constants --- */
#define MPS_PI 3.14159265358979323846
#define MPS_E  2.71828182845904523536

/* --- System Functions --- */
static inline const char* mps_env(const char* key) {
    const char* val = getenv(key);
    return val ? val : "";
}

static inline void mps_exit(int code) {
    exit(code);
}

static inline void mps_sleep(int ms) {
#ifdef _WIN32
    Sleep(ms);
#else
    usleep(ms * 1000);
#endif
}

#endif // MPS_RUNTIME_H

