const input = document.getElementById('todo-input');
const addBtn = document.getElementById('add-btn');
const list = document.getElementById('todo-list');
const countSpan = document.getElementById('todo-count');
const clearBtn = document.getElementById('clear-btn');

let todoCount = 0;

function updateCount() {
    countSpan.textContent = todoCount + ' items';
}

function createTodoItem(text) {
    const li = document.createElement('li');
    li.className = 'todo-item';

    const span = document.createElement('span');
    span.textContent = text;
    span.addEventListener('click', function() {
        li.classList.toggle('completed');
    });

    const delBtn = document.createElement('button');
    delBtn.className = 'delete-btn';
    delBtn.textContent = 'Delete';
    delBtn.addEventListener('click', function() {
        list.removeChild(li);
        todoCount = todoCount - 1;
        updateCount();
    });

    li.appendChild(span);
    li.appendChild(delBtn);
    return li;
}

addBtn.addEventListener('click', function() {
    const text = input.value.trim();
    if (text !== '') {
        const item = createTodoItem(text);
        list.appendChild(item);
        todoCount = todoCount + 1;
        updateCount();
        input.value = '';
    }
});

clearBtn.addEventListener('click', function() {
    todoCount = 0;
    updateCount();
    list.textContent = "0";
});
