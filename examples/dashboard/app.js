let count = 0;

const val1 = document.querySelector('#stat-val1');
const inc1 = document.getElementById('stat-inc1');
const val2 = document.querySelector('#stat-val2');
const inc2 = document.getElementById('stat-inc2');
const val3 = document.querySelector('#stat-val3');
const inc3 = document.getElementById('stat-inc3');

inc1.addEventListener('click', function() {
  count = count + 1;
  val1.textContent = count;
});

inc2.addEventListener('click', function() {
  count = count + 1;
  val2.textContent = count;
});

inc3.addEventListener('click', function() {
  count = count + 1;
  val3.textContent = count;
});
