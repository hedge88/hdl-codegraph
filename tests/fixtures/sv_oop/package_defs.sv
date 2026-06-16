package my_pkg;

    typedef enum logic [1:0] {
        IDLE  = 2'b00,
        BUSY  = 2'b01,
        DONE  = 2'b10,
        ERROR = 2'b11
    } state_t;

    typedef struct packed {
        logic [31:0] addr;
        logic [63:0] data;
        logic        valid;
    } mem_txn_t;

    function automatic int unsigned calc_parity(input logic [7:0] data);
        return ^data;
    endfunction

endpackage

package another_pkg;
    import my_pkg::*;

    parameter MAX_DEPTH = 16;

endpackage
