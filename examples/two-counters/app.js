let count = 0;

const displayA = document.querySelector('.display-a');
const incA = document.getElementById('inc-a');
const resetA = document.getElementById('reset-a');

const displayB = document.querySelector('.display-b');
const incB = document.getElementById('inc-b');
const resetB = document.getElementById('reset-b');

incA.addEventListener('click', function() {
  count = count + 1;
  displayA.textContent = count;
});

resetA.addEventListener('click', function() {
  displayA.textContent = "0";
});

incB.addEventListener('click', function() {
  count = count + 1;
  displayB.textContent = count;
});

resetB.addEventListener('click', function() {
  displayB.textContent = "0";
});
