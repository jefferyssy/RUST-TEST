// Counter App — 交互逻辑
// 编译为 Rust EventListener + DOM API 调用

let count = 0;
const display = document.querySelector('.display');
const btn = document.getElementById('inc-btn');

btn.addEventListener('click', function() {
  count = count + 1;
  display.textContent = count;
});
