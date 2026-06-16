// Test for the uvm_nonblocking_* TLM direction detection bug.
// The substring "uvm_blocking" is checked before "uvm_nonblocking",
// so nonblocking ports may incorrectly match as Blocking.
class nb_test extends uvm_component;

    uvm_nonblocking_put_port #(my_txn) nb_put_port;
    uvm_nonblocking_get_imp #(my_txn, nb_test) nb_get_imp;
    uvm_blocking_put_port #(my_txn) b_put_port;

    function new(string name, uvm_component parent);
        super.new(name, parent);
        nb_put_port = new("nb_put_port", this);
        nb_get_imp  = new("nb_get_imp", this);
        b_put_port  = new("b_put_port", this);
    endfunction

endclass
