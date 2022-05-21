# TODO

1. Document oslog support
1. Refactor trends functionality to new mj_trends program
1. Update Diesel
   - Diesel now requires Connection to me mutable. Because of the multiple
     stores, this would require multiple mutable references. Combining the
     stores would mean just one mutable reference.
