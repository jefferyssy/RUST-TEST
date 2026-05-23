const display = document.querySelector('#display');
const home = document.getElementById('item-home');
const about = document.getElementById('item-about');
const contact = document.getElementById('item-contact');

home.addEventListener('click', function() {
  display.textContent = "Home Page";
});

about.addEventListener('click', function() {
  display.textContent = "About Us";
});

contact.addEventListener('click', function() {
  display.textContent = "Contact Info";
});
