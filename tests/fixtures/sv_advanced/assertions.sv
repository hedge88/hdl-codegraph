module assertion_demo (
    input  logic       clk,
    input  logic       rst_n,
    input  logic       req,
    input  logic       gnt,
    input  logic [7:0] data,
    output logic [7:0] result
);

    // Sequence definition
    sequence req_gnt_seq;
        req ##1 gnt;
    endsequence

    // Property definition
    property req_gnt_prop;
        @(posedge clk) req |-> ##[1:3] gnt;
    endproperty

    // Concurrent assertions
    assert property (req_gnt_prop) else $error("req not followed by gnt");
    assume property (req_gnt_prop);
    cover property (req_gnt_prop);

    // Covergroup
    covergroup cg @(posedge clk);
        coverpoint data {
            bins low  = {[0:63]};
            bins mid  = {[64:127]};
            bins high = {[128:255]};
        }
        coverpoint req;
        cross data, req;
    endgroup

    cg cov = new();

    // Sequential logic
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            result <= 8'b0;
        else if (req && gnt)
            result <= data;
    end

endmodule
