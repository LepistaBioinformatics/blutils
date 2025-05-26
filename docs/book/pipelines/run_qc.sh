#!/bin/bash

# Get input forward and reverse files path from command line

forward=$1
reverse=$2

# Get the base name of the forward file

base=$(basename $forward | sed 's|.fastq||; s|_1||; s|_s||')
output_dir=qc/${base}

# Create paths to the phases lock files

fastqc_dir=${output_dir}/01_fastqc
fastqc_lock_file=${fastqc_dir}/lock.lock
mkdir -p ${fastqc_dir}

trimming_dir=${output_dir}/02_trimming
trimming_lock_file=${trimming_dir}/lock.lock
mkdir -p ${trimming_dir}

merge_dir=${output_dir}/03_merge
merge_lock_file=${merge_dir}/lock.lock
mkdir -p ${merge_dir}

dereplicate_dir=${output_dir}/04_dereplicate
dereplicate_lock_file=${dereplicate_dir}/lock.lock
mkdir -p ${dereplicate_dir}

denoise_dir=${output_dir}/05_denoise
denoise_lock_file=${denoise_dir}/lock.lock
mkdir -p ${denoise_dir}

chimera_dir=${output_dir}/06_chimera
chimera_lock_file=${chimera_dir}/lock.lock
mkdir -p ${chimera_dir}

# Run fastqc on the forward and reverse files

if [ ! -f ${fastqc_lock_file} ]; then
    fastqc -o ${fastqc_dir} ${forward} ${reverse}

    touch ${fastqc_lock_file}
fi

# Run trimming on the forward and reverse files

if [ ! -f ${trimming_lock_file} ]; then
    trimmomatic_dir=$(realpath deps/trimmomatic-0.39)

    java -jar ${trimmomatic_dir}/trimmomatic-0.39.jar \
        PE \
        -threads 8 \
        -phred33 \
        -quiet \
        ${forward} \
        ${reverse} \
        -baseout ${trimming_dir}/${base}.fastq.gz \
        ILLUMINACLIP:${trimmomatic_dir}/adapters/TruSeq3-PE.fa:2:30:10 \
        LEADING:3 \
        TRAILING:3 \
        SLIDINGWINDOW:5:20 \
        MINLEN:120

    touch ${trimming_lock_file}
fi

# Merge the forward and reverse files

if [ ! -f ${merge_lock_file} ]; then
    vsearch \
        --fastq_mergepairs \
        ${trimming_dir}/${base}_1P.fastq.gz \
        --reverse \
        ${trimming_dir}/${base}_2P.fastq.gz \
        --fastaout ${merge_dir}/${base}.merged.fasta \
        --fastqout_notmerged_fwd \
        ${merge_dir}/${base}.unmerged.forward.fastq \
        --fastqout_notmerged_rev \
        ${merge_dir}/${base}.unmerged.reverse.fastq \
        --log \
        ${merge_dir}/${base}.merge.log

    touch ${merge_lock_file}
fi

# Dereplicate the merged file

if [ ! -f ${dereplicate_lock_file} ]; then
    vsearch \
        --derep_prefix \
        ${merge_dir}/${base}.merged.fasta \
        --output \
        ${dereplicate_dir}/${base}.dereplicated.fasta \
        --relabel_md5 \
        --sizeout \
        --minuniquesize 2 \
        --log \
        ${dereplicate_dir}/${base}.dereplicate.log

    touch ${dereplicate_lock_file}
fi

# Denoise the dereplicated file

if [ ! -f ${denoise_lock_file} ]; then
    vsearch \
        --cluster_unoise \
        ${dereplicate_dir}/${base}.dereplicated.fasta \
        --minsize \
        2 \
        --unoise_alpha \
        2.0 \
        --id 0.97 \
        --centroids \
        ${denoise_dir}/${base}.denoised.fasta \
        --relabel \
        ${base}. \
        --sizein \
        --sizeout \
        --log \
        ${denoise_dir}/${base}.denoise.log

    touch ${denoise_lock_file}
fi

# Remove chimeras from the denoised file

if [ ! -f ${chimera_lock_file} ]; then
    vsearch \
        --uchime_denovo \
        ${denoise_dir}/${base}.denoised.fasta \
        --abskew \
        1.5 \
        --nonchimeras \
        ${chimera_dir}/${base}.nonchimeras.fasta \
        --fasta_width \
        0 \
        --sizein \
        --sizeout \
        --log \
        ${chimera_dir}/${base}.chimera.log

    sed -i 's|;size=|_size_|' ${chimera_dir}/${base}.nonchimeras.fasta

    touch ${chimera_lock_file}
fi
