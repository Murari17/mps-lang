#ifndef MPS_RUNTIME_H
#define MPS_RUNTIME_H

#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdarg.h>
#include <math.h>
#include <string.h>
#include <setjmp.h>

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
    } value;
};
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
#endif

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

/* --- Native Fixed-Size Windows Thread Pool for Async/Await --- */
typedef struct MPS_Task {
    void (*fn)(void*);
    void* arg;
    bool completed;
    CONDITION_VARIABLE cv;
    CRITICAL_SECTION cs;
    struct MPS_Task* next;
} MPS_Task;

#ifdef MPS_RUNTIME_IMPL
static MPS_Task* queue_head = NULL;
static MPS_Task* queue_tail = NULL;
static CRITICAL_SECTION pool_cs;
static CONDITION_VARIABLE pool_cv;
static bool pool_shutdown = false;
static HANDLE pool_threads[4];
static int pool_thread_count = 4;
static bool pool_initialized = false;

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
#endif

static inline void mps_pool_init() {
#ifdef MPS_RUNTIME_IMPL
    if (pool_initialized) return;
    InitializeCriticalSection(&pool_cs);
    InitializeConditionVariable(&pool_cv);
    for (int i = 0; i < pool_thread_count; i++) {
        pool_threads[i] = CreateThread(NULL, 0, worker_thread, NULL, 0, NULL);
    }
    pool_initialized = true;
#endif
}

static inline void mps_pool_submit(MPS_Task* task) {
#ifdef MPS_RUNTIME_IMPL
    mps_pool_init();
    task->completed = false;
    InitializeConditionVariable(&task->cv);
    InitializeCriticalSection(&task->cs);
    task->next = NULL;

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
    (void)task;
#endif
}

static inline void mps_task_await(MPS_Task* task) {
    EnterCriticalSection(&task->cs);
    while (!task->completed) {
        SleepConditionVariableCS(&task->cv, &task->cs, INFINITE);
    }
    LeaveCriticalSection(&task->cs);
}

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

static inline PyObject* PyObject_GetItem(PyObject* obj, PyObject* key) {
    if (obj == NULL || key == NULL) return NULL;
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
typedef struct {
    double* data;
    int rows;
    int cols;
} MPSMatrix;

static inline MPSMatrix* matrix_new(int rows, int cols) {
    MPSMatrix* m = (MPSMatrix*)malloc(sizeof(MPSMatrix));
    m->rows = rows;
    m->cols = cols;
    m->data = (double*)calloc(rows * cols, sizeof(double));
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
    
    double* b_T = (double*)malloc(sizeof(double) * b->rows * b->cols);
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
    free(b_T);
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

#define print(X) _Generic((X), \
    const char*: _print_string, \
    char*: _print_string, \
    int: _print_int, \
    double: _print_double, \
    float: _print_double, \
    bool: _print_bool, \
    PyObject*: _print_pyobject, \
    MPSMatrix*: _print_matrix \
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

// Casting
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

